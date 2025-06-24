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
use crate::herder::{
    report_errors, ExitRequest, ScriptCompletions, ScriptHerder, ScriptQueue, ScriptReceipt,
    ScriptRunKind, ScriptStep,
};
use anyhow::Result;
use bevy_ecs::{
    prelude::*,
    query::QueryData,
    schedule::{LogLevel, ScheduleBuildSettings},
    system::Resource,
};
use nitrous::{
    inject_nitrous_resource, method, Heap, HeapMut, LocalNamespace, NamedEntityMut,
    NitrousResource, NitrousScript, ScriptComponent, ScriptResource,
};
use parking_lot::Mutex;
use std::{fs, path::PathBuf, sync::Arc};

/// Interface for extending the Runtime.
pub trait Extension {
    type Opts;
    fn init(runtime: &mut Runtime, opts: Self::Opts) -> Result<()>;
}

// Systems may be scheduled to run at startup.
// The startup scripts run in RunScript.
#[derive(SystemSet, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum StartupSet {
    Main,
}

// Systems may be scheduled to run after the mainloop, for cleanup.
#[derive(SystemSet, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ShutdownSet {
    Cleanup,
}

/// The simulation schedule should be used for "pure" entity to entity work and update of
/// a handful of game related resources, rather than communicating with the GPU.
#[derive(SystemSet, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SimSet {
    /// Clobber the events buffer
    FlushEvents,
    /// Pre-script parallel phase.
    Input,
    /// Fully serial phase where scripts run.
    RunScript,
    /// Simulate with latest input state.
    Simulate,
}

#[derive(SystemSet, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum RuntimeSet {
    ClearCompletions,
}

#[derive(Debug, Default, NitrousResource)]
#[resource(name = "runtime")]
pub struct RuntimeResource {
    sim_number: u64,
    frame_number: u64,
}

#[inject_nitrous_resource]
impl RuntimeResource {
    #[method]
    fn exec(&self, filename: &str, mut heap: HeapMut) -> Result<()> {
        let script_text = fs::read_to_string(PathBuf::from(filename))?;
        heap.resource_mut::<ScriptQueue>()
            .run_interactive(&script_text);
        Ok(())
    }

    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    pub fn sim_number(&self) -> u64 {
        self.sim_number
    }

    pub fn state(&self) -> (u64, u64) {
        (self.sim_number, self.frame_number)
    }
}

pub struct Runtime {
    heap: Heap,
    startup_schedule: Schedule,
    sim_schedule: Schedule,
    frame_schedule: Schedule,
    shutdown_schedule: Schedule,
    dump_schedules: bool,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        let mut settings = ScheduleBuildSettings::new();
        settings.ambiguity_detection = LogLevel::Error;
        settings.hierarchy_detection = LogLevel::Error;
        settings.use_shortnames = true;
        settings.report_sets = true;

        let mut startup_schedule = Schedule::default();
        startup_schedule.set_build_settings(settings.clone());
        startup_schedule.add_systems(
            ScriptHerder::sys_run_startup_scripts
                .pipe(report_errors)
                .in_set(StartupSet::Main),
        );

        let mut sim_schedule = Schedule::default();
        sim_schedule.set_build_settings(settings.clone());
        sim_schedule.configure_sets((SimSet::Input, SimSet::RunScript, SimSet::Simulate).chain());
        sim_schedule.add_systems(
            ScriptHerder::sys_run_sim_scripts
                .pipe(report_errors)
                .in_set(SimSet::RunScript)
                .in_set(ScriptStep::Run),
        );
        sim_schedule.add_systems(
            bevy_ecs::event::event_update_system
                .in_set(SimSet::FlushEvents)
                .before(SimSet::Input),
        );

        let mut frame_schedule = Schedule::default();
        frame_schedule.set_build_settings(settings);
        frame_schedule
            .add_systems(ScriptHerder::sys_clear_completions.in_set(RuntimeSet::ClearCompletions));

        let shutdown_schedule = Schedule::default();

        let mut runtime = Self {
            heap: Heap::default(),
            startup_schedule,
            sim_schedule,
            frame_schedule,
            shutdown_schedule,
            dump_schedules: false,
        };

        runtime
            .inject_resource(ExitRequest::Continue)?
            .inject_resource(ScriptHerder::default())?
            .inject_resource(ScriptCompletions::default())?
            .inject_resource(ScriptQueue::default())?
            .inject_resource(RuntimeResource::default())?;

        Ok(runtime)
    }

    #[inline]
    pub fn heap(&self) -> &Heap {
        &self.heap
    }

    #[inline]
    pub fn heap_mut(&mut self) -> HeapMut {
        HeapMut::wrap(self.heap.world_mut())
    }

    #[inline]
    pub fn run_sim_once(&mut self) {
        self.heap.resource_mut::<RuntimeResource>().sim_number += 1;
        self.sim_schedule.run(self.heap.world_mut());
    }

    #[inline]
    pub fn run_frame_once(&mut self) {
        self.heap.resource_mut::<RuntimeResource>().frame_number += 1;
        self.frame_schedule.run(self.heap.world_mut());
    }

    #[inline]
    pub fn sim_number(&self) -> u64 {
        self.heap.resource::<RuntimeResource>().sim_number
    }

    #[inline]
    pub fn frame_number(&self) -> u64 {
        self.heap.resource::<RuntimeResource>().frame_number
    }

    #[inline]
    pub fn run_startup(&mut self) {
        self.startup_schedule.run(self.heap.world_mut());

        if self.dump_schedules {
            self.dump_schedules = false;
            self.dump_startup_schedule();
            self.dump_sim_schedule();
            self.dump_frame_schedule();
            self.dump_shutdown_schedule();
        }
    }

    #[inline]
    pub fn run_shutdown(&mut self) {
        self.shutdown_schedule.run(self.heap.world_mut());
    }

    #[inline]
    pub fn set_dump_schedules_on_startup(&mut self) {
        self.dump_schedules = true;
    }

    #[inline]
    pub fn dump_startup_schedule(&self) {
        // dump_schedule(
        //     self.heap.world(),
        //     &self.startup_schedule,
        //     &PathBuf::from("startup_schedule.dot"),
        // );
    }

    #[inline]
    pub fn dump_sim_schedule(&self) {
        // dump_schedule(
        //     self.heap.world(),
        //     &self.sim_schedule,
        //     &PathBuf::from("sim_schedule.dot"),
        // );
    }

    #[inline]
    pub fn dump_frame_schedule(&self) {
        // dump_schedule(
        //     self.heap.world(),
        //     &self.frame_schedule,
        //     &PathBuf::from("frame_schedule.dot"),
        // );
    }

    #[inline]
    pub fn dump_shutdown_schedule(&self) {
        // dump_schedule(
        //     self.heap.world(),
        //     &self.shutdown_schedule,
        //     &PathBuf::from("shutdown_schedule.dot"),
        // );
    }

    #[inline]
    pub fn load_extension<T>(&mut self) -> Result<&mut Self>
    where
        T: Extension,
        T::Opts: Default,
    {
        T::init(self, T::Opts::default())?;
        Ok(self)
    }

    #[inline]
    pub fn with_extension<T>(mut self) -> Result<Self>
    where
        T: Extension,
        T::Opts: Default,
    {
        T::init(&mut self, T::Opts::default())?;
        Ok(self)
    }

    #[inline]
    pub fn load_extension_with<T>(&mut self, opts: T::Opts) -> Result<&mut Self>
    where
        T: Extension,
    {
        T::init(self, opts)?;
        Ok(self)
    }

    #[inline]
    pub fn with_extension_with<T>(mut self, opts: T::Opts) -> Result<Self>
    where
        T: Extension,
    {
        T::init(&mut self, opts)?;
        Ok(self)
    }

    pub fn add_script_system<M>(&mut self, system: impl IntoSystemConfigs<M>) -> &mut Self {
        self.sim_schedule
            .add_systems(system.in_set(SimSet::RunScript));
        self
    }

    pub fn add_input_system<M>(&mut self, system: impl IntoSystemConfigs<M>) -> &mut Self {
        self.sim_schedule.add_systems(system.in_set(SimSet::Input));
        self
    }

    pub fn add_input_systems<M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self {
        self.sim_schedule.add_systems(systems.in_set(SimSet::Input));
        self
    }

    pub fn add_sim_system<M>(&mut self, system: impl IntoSystemConfigs<M>) -> &mut Self {
        self.sim_schedule
            .add_systems(system.in_set(SimSet::Simulate));
        self
    }

    pub fn add_sim_systems<M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self {
        self.sim_schedule
            .add_systems(systems.in_set(SimSet::Simulate));
        self
    }

    pub fn add_frame_system<M>(&mut self, system: impl IntoSystemConfigs<M>) -> &mut Self {
        self.frame_schedule.add_systems(system);
        self
    }

    pub fn add_frame_systems<M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self {
        self.frame_schedule.add_systems(systems);
        self
    }

    pub fn add_startup_system<M>(&mut self, system: impl IntoSystemConfigs<M>) -> &mut Self {
        self.startup_schedule.add_systems(system);
        self
    }

    // Script passthrough

    #[inline]
    pub fn run_interactive(&mut self, script_text: &str) -> Result<ScriptReceipt> {
        self.resource_mut::<ScriptHerder>()
            .run_interactive(script_text)
    }

    #[inline]
    pub fn run_string(&mut self, script_text: &str) -> Result<ScriptReceipt> {
        self.resource_mut::<ScriptHerder>().run_string(script_text)
    }

    #[inline]
    pub fn run<N: Into<NitrousScript>>(&mut self, script: N) -> ScriptReceipt {
        self.resource_mut::<ScriptHerder>()
            .run(script, ScriptRunKind::Precompiled)
    }

    #[inline]
    pub fn run_with_locals<N: Into<NitrousScript>>(
        &mut self,
        locals: LocalNamespace,
        script: N,
    ) -> ScriptReceipt {
        self.resource_mut::<ScriptHerder>().run_with_locals(
            Arc::new(Mutex::new(locals)),
            script,
            ScriptRunKind::Precompiled,
        )
    }

    // Heap passthrough

    #[inline]
    pub fn spawn<S>(&mut self, name: S) -> Result<NamedEntityMut>
    where
        S: Into<String>,
    {
        self.heap.spawn(name)
    }

    #[inline]
    pub fn get_by_id<T>(&self, entity: Entity) -> &T
    where
        T: Component + ScriptComponent + 'static,
    {
        self.heap.get_by_id::<T>(entity)
    }

    #[inline]
    pub fn get_by_id_mut<T>(&mut self, entity: Entity) -> Result<Mut<T>>
    where
        T: Component + ScriptComponent + 'static,
    {
        self.heap.get_by_id_mut::<T>(entity)
    }

    #[inline]
    pub fn inject_resource<T>(&mut self, value: T) -> Result<&mut Self>
    where
        T: Resource + ScriptResource + 'static,
    {
        self.heap.inject_resource(value)?;
        Ok(self)
    }

    #[inline]
    pub fn insert_non_send_resource<T: 'static>(&mut self, value: T) -> &mut Self {
        self.heap.insert_non_send_resource(value);
        self
    }

    #[inline]
    pub fn non_send_resource<T: 'static>(&self) -> &T {
        self.heap.non_send_resource()
    }

    #[inline]
    pub fn non_send_resource_mut<T: 'static>(&mut self) -> Mut<T> {
        self.heap.non_send_resource_mut()
    }

    #[inline]
    pub fn maybe_resource<T: Resource>(&self) -> Option<&T> {
        self.heap.maybe_resource()
    }

    #[inline]
    pub fn resource<T: Resource>(&self) -> &T {
        self.heap.resource::<T>()
    }

    #[inline]
    pub fn resource_mut<T: Resource>(&mut self) -> Mut<T> {
        self.heap.resource_mut::<T>()
    }

    #[inline]
    pub fn resource_by_name(&mut self, name: &str) -> &dyn ScriptResource {
        self.heap.resource_by_name(name)
    }

    #[inline]
    pub fn remove_resource<T: ScriptResource>(&mut self) -> Option<T> {
        self.heap.remove_resource::<T>()
    }

    #[inline]
    pub fn resource_scope<T: Resource, U>(&mut self, f: impl FnOnce(HeapMut, Mut<T>) -> U) -> U {
        self.heap.resource_scope(f)
    }

    #[inline]
    pub fn register_event<E: Event>(&mut self) {
        self.heap.register_event::<E>();
    }

    #[inline]
    pub fn deregister_event<E: Event>(&mut self) {
        self.heap.deregister_event::<E>();
    }

    #[inline]
    pub fn query<Q>(&mut self) -> QueryState<Q, ()>
    where
        Q: QueryData,
    {
        self.heap.query::<Q>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let _ = Runtime::new()?;
        Ok(())
    }
}
