// This file is part of Nitrogen.
//
// Nitrogen is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Nitrogen is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Nitrogen.  If not, see <http://www.gnu.org/licenses/>.
use anyhow::{bail, Result};
use bevy_ecs::prelude::*;
use itertools::*;
use log::{debug, error, info, trace, warn};
use nitrous::{
    inject_nitrous_resource, method, ExecutionContext, HeapMut, LocalNamespace, NitrousExecutor,
    NitrousResource, NitrousScript, Value, WorldIndex, YieldState,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;

pub const GUIDE: &str = r#"
Welcome to the Nitrogen Terminal
--------------------------------
From here, you can tweak and investigate every aspect of the game.

The command `list()` may be used at the top level, or on any item, to get a list
of all items that can be accessed there. Use `help()` to show this message again.

Engine "resources" are accessed with the name of the resource followed by a dot,
followed by the name of a property or method on the resource. Methods may be called
by adding a pair of parentheses after.

Examples:
   terrain.toggle_pin_camera(true)

Named game "entities" are accessed with an @ symbol, followed by the name of the
entity, followed by a dot, followed by the name of a "component" on the entity,
followed by another dot, followed by the name of a property or method on that
component. As with resources, methods are called by appending parentheses.

Examples:
    @player.throttle.set_detent(4)
"#;

#[derive(Copy, Clone, Debug, Eq, PartialEq, NitrousResource)]
pub enum ExitRequest {
    Exit,
    Continue,
}

#[inject_nitrous_resource]
impl ExitRequest {
    #[method]
    pub fn request_exit(&mut self) {
        warn!("Requesting Exit");
        *self = ExitRequest::Exit;
    }

    #[method]
    pub fn still_running(&self) -> bool {
        *self == ExitRequest::Continue
    }
}

pub trait Scriptable {
    fn run(&mut self, script_text: &str);
}

/// Sometimes scripts need to run other scripts. But since we're inside Herder,
/// it's not available to push to directly. Herder will check this resource at
/// the start of it's run phase and start anything that's been queued.
#[derive(Debug, Default, NitrousResource)]
pub struct ScriptQueue {
    queue: Vec<String>,
}

#[inject_nitrous_resource]
impl ScriptQueue {
    #[method]
    fn active(&self) -> i64 {
        self.queue.len() as i64
    }

    pub fn run_interactive<S: Into<String>>(&mut self, script_text: S) {
        self.queue.push(script_text.into());
    }

    pub fn run_string<S: Into<String>>(&mut self, script_text: S) {
        self.queue.push(script_text.into());
    }
}

impl Scriptable for ScriptQueue {
    fn run(&mut self, script_text: &str) {
        trace!("queue run: {script_text}");
        self.queue.push(script_text.to_owned());
    }
}

#[derive(SystemSet, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ScriptStep {
    Run,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ScriptRunKind {
    Interactive,
    String,
    Precompiled,
    Binding,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ScriptRunPhase {
    Startup,
    Sim,
}

#[derive(Clone, Debug)]
pub struct ExecutionMetadata {
    context: ExecutionContext,
    kind: ScriptRunKind,
    receipt: ScriptReceipt,
}

impl ExecutionMetadata {
    pub fn kind(&self) -> ScriptRunKind {
        self.kind
    }

    pub fn context(&self) -> &ExecutionContext {
        &self.context
    }

    pub fn maybe_add_builtins(&mut self) {
        if self.context.has_started() {
            // Don't repeat for yielded scripts
            return;
        }

        if self.kind == ScriptRunKind::Interactive {
            self.context.locals_mut().put_if_absent(
                "resources",
                Value::RustMethod(Arc::new(|_args, heap: HeapMut| {
                    // Note: itersperse from itertools conflicts with a reserved name
                    #[allow(unstable_name_collisions)]
                    let item_list: Value = (String::from("  ")
                        + &heap
                            .resource::<WorldIndex>()
                            .resource_names()
                            .intersperse("\n  ")
                            .collect::<String>())
                        .into();
                    Ok(item_list)
                })),
            );
            self.context.locals_mut().put_if_absent(
                "entities",
                Value::RustMethod(Arc::new(|_args, heap: HeapMut| {
                    // Note: itersperse from itertools conflicts with a reserved name
                    #[allow(unstable_name_collisions)]
                    let item_list: Value = (String::from("  @")
                        + &heap
                            .resource::<WorldIndex>()
                            .entity_names()
                            .intersperse("\n  @")
                            .collect::<String>())
                        .into();
                    Ok(item_list)
                })),
            );
            self.context.locals_mut().put_if_absent(
                "list",
                Value::RustMethod(Arc::new(|_args, heap: HeapMut| {
                    // Note: itersperse from itertools conflicts with a reserved name
                    #[allow(unstable_name_collisions)]
                    let item_list: Value = (String::new()
                        + "Resources:\n  "
                        + &heap
                            .resource::<WorldIndex>()
                            .resource_names()
                            .intersperse("\n  ")
                            .collect::<String>()
                        + "\nEntities:\n  @"
                        + &heap
                            .resource::<WorldIndex>()
                            .entity_names()
                            .intersperse("\n  @")
                            .collect::<String>())
                        .into();
                    Ok(item_list)
                })),
            );
            self.context.locals_mut().put_if_absent(
                "help",
                Value::RustMethod(Arc::new(move |_, _| Ok(GUIDE.to_owned().into()))),
            );
        }
    }
}

#[derive(Clone, Debug)]
pub enum ScriptResult {
    Ok(Value),
    Err(String),
}

impl ScriptResult {
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Ok(_) => None,
            Self::Err(s) => Some(s.as_str()),
        }
    }

    pub fn unwrap(self) -> Value {
        match self {
            Self::Ok(v) => v,
            Self::Err(e) => panic!("ScriptResult::unwrap: {e}"),
        }
    }
}

/// Returned by run_script so that script results can be correlated.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ScriptReceipt(usize);

/// Report on script execution result.
#[derive(Clone, Debug)]
pub struct ScriptCompletion {
    pub receipt: ScriptReceipt,
    pub result: ScriptResult,
    pub phase: ScriptRunPhase,
    pub meta: ExecutionMetadata,
}

impl ScriptCompletion {
    pub fn unwrap(&self) -> Value {
        self.result.clone().unwrap()
    }
}

/// A set of script execution results, indented for use as a resource for other systems.
#[derive(Default, NitrousResource)]
pub struct ScriptCompletions {
    complete: Vec<ScriptCompletion>,
}

#[inject_nitrous_resource]
impl ScriptCompletions {
    #[method]
    pub fn count(&self) -> i64 {
        self.complete.len() as i64
    }

    pub fn iter(&self) -> impl Iterator<Item = &ScriptCompletion> {
        self.complete.iter()
    }
}

/// Additional reporting to make globally.
pub static ERROR_REPORTS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Show an error in the normal way.
#[macro_export]
macro_rules! report {
    ($fmt: expr $(, $($arg:tt)*)?) => {
        $crate::report_info(format!($fmt, $($($arg)*)?));
    };
}

/// Like try!, but in the context of bevy systems. Call report_err!, then return ().
#[macro_export]
macro_rules! try_report_in_system {
    ($expr:expr $(,)?) => {
        match $expr {
            $crate::reexport::result::Result::Ok(val) => val,
            $crate::reexport::result::Result::Err(err) => {
                $crate::report_err(err);
                return;
            }
        }
    };
}

/// Report an error in a consistent way.
pub fn report_err(err: anyhow::Error) {
    error!("{}\n{}", err, err.backtrace());
    ERROR_REPORTS.lock().push(err.to_string());
}

pub fn report_info<S: ToString>(msg: S) {
    info!("{}", msg.to_string());
    ERROR_REPORTS.lock().push(msg.to_string())
}

// Use with chain to turn a system returning a Result into a functioning system.
pub fn report_errors(In(result): In<Result<()>>) {
    try_report_in_system!(result);
}

/// Manage script execution state.
#[derive(Default, NitrousResource)]
#[resource(name = "script")]
pub struct ScriptHerder {
    receipt_offset: usize,
    gthread: Vec<ExecutionMetadata>,
    interactive_locals: Arc<Mutex<LocalNamespace>>,
}

impl Scriptable for ScriptHerder {
    fn run(&mut self, script_text: &str) {
        trace!("run: {}", script_text);
        self.run_string(script_text).ok();
    }
}

#[inject_nitrous_resource]
impl ScriptHerder {
    #[method(name = "exec")]
    fn exec_n2o(&mut self, script_text: &str) -> Result<()> {
        self.run_with_locals(
            self.interactive_locals.clone(),
            NitrousScript::compile(script_text)?,
            ScriptRunKind::Interactive,
        );
        Ok(())
    }

    #[inline]
    pub fn run_interactive(&mut self, script_text: &str) -> Result<ScriptReceipt> {
        Ok(self.run_with_locals(
            self.interactive_locals.clone(),
            NitrousScript::compile(script_text)?,
            ScriptRunKind::Interactive,
        ))
    }

    pub fn run_string(&mut self, script_text: &str) -> Result<ScriptReceipt> {
        trace!("run_string: {}", script_text);
        Ok(self.run(NitrousScript::compile(script_text)?, ScriptRunKind::String))
    }

    #[inline]
    pub fn run<N: Into<NitrousScript>>(&mut self, script: N, kind: ScriptRunKind) -> ScriptReceipt {
        self.run_with_locals(Default::default(), script, kind)
    }

    #[inline]
    pub fn run_binding<N: Into<NitrousScript>>(
        &mut self,
        locals: LocalNamespace,
        script: N,
    ) -> ScriptReceipt {
        self.run_with_locals(Arc::new(Mutex::new(locals)), script, ScriptRunKind::Binding)
    }

    #[inline]
    pub fn run_with_locals<N: Into<NitrousScript>>(
        &mut self,
        locals: Arc<Mutex<LocalNamespace>>,
        script: N,
        kind: ScriptRunKind,
    ) -> ScriptReceipt {
        self.receipt_offset += 1;
        let receipt = ScriptReceipt(self.receipt_offset);
        self.gthread.push(ExecutionMetadata {
            context: ExecutionContext::new(locals, script.into()),
            kind,
            receipt,
        });
        receipt
    }

    #[inline]
    pub(crate) fn sys_run_startup_scripts(world: &mut World) -> Result<()> {
        world.resource_scope(|world, mut herder: Mut<ScriptHerder>| {
            herder._run_scripts(HeapMut::wrap(world), ScriptRunPhase::Startup)
        })?;
        trace!("clearing startup script completions");
        world.resource_mut::<ScriptCompletions>().complete.clear();
        Ok(())
    }

    #[inline]
    pub(crate) fn sys_run_sim_scripts(world: &mut World) -> Result<()> {
        world.resource_scope(|world, mut herder: Mut<ScriptHerder>| {
            herder._run_scripts(HeapMut::wrap(world), ScriptRunPhase::Sim)
        })?;
        Ok(())
    }

    pub(crate) fn sys_clear_completions(mut completions: ResMut<ScriptCompletions>) {
        // This runs at frame schedule, whereas scripts may run each sim step.
        // trace!("clearing script completions");
        completions.complete.clear();
    }

    // Exposed for testing the internals
    pub fn _run_scripts(&mut self, mut heap: HeapMut, phase: ScriptRunPhase) -> Result<()> {
        // Queue up deferred scripts.
        for script in &heap.resource::<ScriptQueue>().queue {
            self.run_interactive(script)?;
        }
        heap.resource_mut::<ScriptQueue>().queue.clear();

        let mut next_gthreads = Vec::with_capacity(self.gthread.capacity());
        for mut meta in self.gthread.drain(..) {
            meta.maybe_add_builtins();
            debug!("about to run: {}", meta.context.script());
            let executor = NitrousExecutor::new(&mut meta.context, heap.as_mut());
            match executor.run_until_yield() {
                Ok(yield_state) => match yield_state {
                    YieldState::Yielded => next_gthreads.push(meta),
                    YieldState::Finished(result) => {
                        info!("{:?}: {} <- {}", phase, result, meta.context.script());
                        heap.resource_mut::<ScriptCompletions>()
                            .complete
                            .push(ScriptCompletion {
                                receipt: meta.receipt,
                                result: ScriptResult::Ok(result),
                                phase,
                                meta,
                            });
                    }
                },
                Err(err) => {
                    heap.resource_mut::<ScriptCompletions>()
                        .complete
                        .push(ScriptCompletion {
                            receipt: meta.receipt,
                            result: ScriptResult::Err(format!("{err}")),
                            phase,
                            meta,
                        });
                    bail!("script failed: {}", err);
                }
            }
        }
        self.gthread = next_gthreads;
        Ok(())
    }
}
