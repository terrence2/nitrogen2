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
// mod dump_schedule;
// mod herder;
// mod runtime;
// mod startup;
mod stdpath;
// mod timestep;

pub mod reexport {
    // pub use log;
    pub use bevy::ecs::error::BevyError;
    pub use std::result;
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr) => {
        if !$cond {
            return Err($crate::reexport::BevyError::from(format!(
                "Ensure failed: {}",
                stringify!($cond)
            )));
        }
    };
    ($cond:expr, $msg:expr) => {
        if !$cond {
            return Err($crate::reexport::BevyError::from(format!(
                "Assertion failed: {}: {}",
                stringify!($cond),
                $msg
            )));
        }
    };
}

pub use crate::{
    // herder::{
    //     report_err, report_errors, report_info, ExecutionMetadata, ExitRequest, ScriptCompletion,
    //     ScriptCompletions, ScriptHerder, ScriptQueue, ScriptReceipt, ScriptResult, ScriptRunKind,
    //     ScriptRunPhase, ScriptStep, Scriptable, ERROR_REPORTS,
    // },
    // runtime::{Extension, Runtime, RuntimeResource, RuntimeSet, ShutdownSet, SimSet, StartupSet},
    // startup::StartupOpts,
    stdpath::{StdPaths, StdPathsPlugin},
    // timestep::{TimeStep, TimeStepStep},
};

// use bevy_ecs::prelude::*;
// use nitrous::{inject_nitrous_component, NitrousComponent};
//
// #[derive(NitrousComponent, Debug, Default)]
// pub struct PlayerMarker;
//
// #[inject_nitrous_component]
// impl PlayerMarker {}
