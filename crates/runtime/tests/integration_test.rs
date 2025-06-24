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
/*
use anyhow::Result;
use bevy::prelude::*;
use nitrous::{
    NitrousComponent, NitrousResource, Value, inject_nitrous_component, inject_nitrous_resource,
    method,
};
use runtime::{Runtime, ScriptCompletions, ScriptHerder, ScriptRunPhase};
use std::collections::HashMap;

#[derive(Debug, NitrousResource)]
pub struct Globals {
    #[property]
    bool_resource: bool,

    #[property]
    int_resource: i64,

    #[property]
    float_resource: f64,

    #[property]
    string_resource: String,
}

impl Default for Globals {
    fn default() -> Self {
        Self {
            bool_resource: true,
            int_resource: 42_i64,
            float_resource: 42_f64,
            string_resource: "Foobar".to_owned(),
        }
    }
}

#[inject_nitrous_resource]
impl Globals {
    #[method]
    fn add_float(&self, v: f64) -> f64 {
        self.float_resource + v
    }

    #[method]
    fn add_int(&self, v: i64) -> i64 {
        self.int_resource + v
    }

    #[method]
    fn add_string(&self, v: &str) -> String {
        self.string_resource.clone() + v
    }

    #[method]
    fn check_bool(&self, v: bool) -> bool {
        self.bool_resource == v
    }
}

#[derive(Debug, NitrousComponent)]
#[component(name = "item")]
pub struct Item {
    #[property]
    float_resource: f64,
}

impl Default for Item {
    fn default() -> Self {
        Self {
            float_resource: 42_f64,
        }
    }
}

#[inject_nitrous_component]
impl Item {
    #[method]
    fn add_float(&self, v: f64) -> f64 {
        self.float_resource + v
    }
}

#[derive(Debug, Default, NitrousComponent)]
pub struct SnakeTest {}

#[inject_nitrous_component]
impl SnakeTest {
    /// This is some help text!
    #[method(name = "shadow")]
    fn shadow_n2o(&self) {}
}

#[test]
fn integration_test() -> Result<()> {
    env_logger::init();

    let mut runtime = Runtime::new()?;
    runtime.inject_resource(Globals::default())?;
    runtime
        .spawn("player")?
        .inject(Item::default())?
        .inject(SnakeTest::default())?;

    // Resource
    runtime.run_string("globals.bool_resource")?;
    runtime.run_string("globals.bool_resource <- False")?;
    let br2 = runtime.run_string("globals.check_bool(False)")?;

    runtime.run_string("globals.int_resource")?;
    runtime.run_string("globals.int_resource <- 2")?;
    let ir2 = runtime.run_string("globals.add_int(2)")?;

    runtime.run_string("globals.float_resource")?;
    runtime.run_string("globals.float_resource <- 2.")?;
    let fr2 = runtime.run_string("globals.add_float(2.)")?;

    runtime.run_string("globals.string_resource")?;
    runtime.run_string("globals.string_resource <- \"Hello\"")?;
    let sr2 = runtime.run_string("globals.add_string(\", World!\")")?;

    // Entity
    runtime.run_string("@player.item.float_resource")?;
    runtime.run_string("@player.item.float_resource <- 2.")?;
    let fe2 = runtime.run_string("@player.item.add_float(2.)")?;

    let se0 = runtime.run_string("@player.snake_test.shadow_n2o()")?;

    runtime.resource_scope(|heap, mut herder: Mut<ScriptHerder>| {
        herder._run_scripts(heap, ScriptRunPhase::Startup)
    })?;

    let mut completions = HashMap::new();
    for completion in runtime.resource::<ScriptCompletions>().iter() {
        completions.insert(completion.receipt, completion);
    }

    // Resource
    assert_eq!(completions[&br2].unwrap(), Value::from_bool(true));
    assert_eq!(completions[&ir2].unwrap(), Value::from_int(4));
    assert_eq!(completions[&fr2].unwrap(), Value::from_float(4_f64));
    assert_eq!(completions[&sr2].unwrap(), Value::from_str("Hello, World!"));

    // Entity
    assert_eq!(completions[&fe2].unwrap(), Value::from_float(4_f64));

    assert_eq!(completions[&se0].unwrap(), Value::True());

    Ok(())
}
*/
