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
use crate::{Extension, Runtime};
use absolute_unit::prelude::*;
use anyhow::Result;
use bevy_ecs::prelude::*;
use hifitime::Epoch;
use nitrous::{inject_nitrous_resource, method, NitrousResource};

#[derive(SystemSet, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TimeStepStep {
    Tick,
}

/// It was the best of timelines, it was the worst of timelines.
/// It was a timeline of real time, it was a timeline of sim time.
/// It was a flexible and real timestep, it was a fixed and constant timestep.
///
/// These timelines are TAI elapsed seconds from the following epochs:
///   * nitrogen boot up (nitrogen_boot_epoch)
///   * real time "now" (real_now_epoch)
///   * sim time "now"
///   * target next sim time now
#[derive(Debug, NitrousResource)]
#[resource(name = "time")]
pub struct TimeStep {
    // Records the instant in time when nitrogen (or at least this resource) booted.
    nitrogen_boot_epoch: Epoch,

    // The real-time "now"
    real_now_epoch: Epoch,

    // The current sim step's "now"
    sim_now_epoch: Epoch,

    // Goal time for next simulation iteration. See comment above advance_frame_times_one_step.
    next_sim_epoch: Epoch,
    sim_step_delta: hifitime::Duration,

    // Record the last frame elapsed to show in frame time dashboards.
    prior_frame_real_elapsed: hifitime::Duration,

    // The current compression factor to apply when matching the sim timeline to the real timeline.
    time_compression: u32,
}

impl Extension for TimeStep {
    type Opts = ();
    fn init(runtime: &mut Runtime, _: ()) -> Result<()> {
        runtime.inject_resource(TimeStep::new_60fps())?;
        runtime.add_input_system(Self::sys_advance_sim_time_one_step.in_set(TimeStepStep::Tick));
        Ok(())
    }
}

#[inject_nitrous_resource]
impl TimeStep {
    pub fn new_60fps() -> Self {
        let now = Epoch::now().expect("working system time");
        let sim_step_delta = hifitime::Duration::from_seconds(1. / 60.);
        Self {
            nitrogen_boot_epoch: now,
            real_now_epoch: now,
            sim_now_epoch: now,
            next_sim_epoch: now + sim_step_delta,
            sim_step_delta,
            prior_frame_real_elapsed: sim_step_delta,
            time_compression: 1,
        }
    }

    #[method]
    pub fn time_compression(&self) -> i64 {
        self.time_compression as i64
    }

    #[method]
    pub fn set_time_compression(&mut self, time_compression: i64) -> Result<()> {
        self.time_compression = u32::try_from(time_compression)?;
        Ok(())
    }

    #[method]
    pub fn next_time_compression(&mut self) {
        if self.time_compression >= 8 {
            self.time_compression = 1;
        } else if self.time_compression >= 4 {
            self.time_compression = 8;
        } else if self.time_compression >= 2 {
            self.time_compression = 4;
        } else if self.time_compression >= 1 {
            self.time_compression = 2;
        } else {
            self.time_compression = 1;
        }
    }

    /// This resource is TimeStep because time is fixed during our key computation passes
    /// and only advances atomically between key moments. Those key moments are in between
    /// frames and in-between sim steps.
    ///
    /// This routine is to advance time one frame step. Frame steps should align with and
    /// skew as little as possible from "real" time. Since sim time advances at a rate
    /// proportional to real time, we need to figure out how much simulation work to do
    /// at this point. However, we are not at an appropriate point to advance sim time, so
    /// this only estimates how far we should advance sim time before our next frame.
    ///
    /// Call once per frame to keep the sim updated.
    pub fn advance_frame_times_one_step(&mut self) {
        let now = Epoch::now().expect("a valid UTC timestamp");
        let real_elapsed = now - self.real_now_epoch;
        let sim_elapsed_f64 = real_elapsed.to_seconds() * self.time_compression as f64;
        let sim_elapsed = hifitime::Duration::from_seconds(sim_elapsed_f64);
        self.prior_frame_real_elapsed = real_elapsed;
        self.next_sim_epoch += sim_elapsed;
        self.real_now_epoch = now;
    }

    pub fn need_sim_step(&self) -> bool {
        self.sim_now_epoch + self.sim_step_delta < self.next_sim_epoch
    }

    pub fn sys_advance_sim_time_one_step(mut timestep: ResMut<TimeStep>) {
        let dt = timestep.sim_step_delta;
        timestep.sim_now_epoch += dt;
    }

    pub fn sim_total_elapsed_duration(&self) -> hifitime::Duration {
        self.sim_now_epoch - self.nitrogen_boot_epoch
    }

    /// Epoch at which nitrogen started up.
    pub fn boot_epoch(&self) -> &Epoch {
        &self.nitrogen_boot_epoch
    }

    /// Epoch of "now" in real wall clock time.
    pub fn real_now_epoch(&self) -> &Epoch {
        &self.real_now_epoch
    }

    pub fn sim_now_epoch(&self) -> &Epoch {
        &self.sim_now_epoch
    }

    /// Return the real time that has elapsed since the program started.
    #[method]
    pub fn real_time_elapsed_since_boot(&self) -> Time<Seconds> {
        seconds!(self.real_now_epoch().to_unix_seconds() - self.boot_epoch().to_unix_seconds())
    }

    /// Return the time between the prior frame's "frame step" calls.
    pub fn prior_frame_real_duration(&self) -> Time<Seconds> {
        seconds!(self.prior_frame_real_elapsed.to_seconds())
    }

    /// Returns the fixed simulation time step of this TimeStep
    pub fn fixed_sim_step(&self) -> &hifitime::Duration {
        &self.sim_step_delta
    }

    /// Same as fixed_sim_step but in an absolute_unit compatible format.
    pub fn fixed_sim_step_time(&self) -> Time<Seconds> {
        seconds!(self.sim_step_delta.to_seconds())
    }
}
