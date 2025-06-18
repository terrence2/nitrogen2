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
pub(crate) mod scalar;
pub(crate) mod unit;
pub(crate) mod vector;

// References: https://www.ferrisellis.com/content/rust-implementing-units-for-types/#constructing-a-unifying-base-struct-and-trait

// Re-export for macro usage
pub use anyhow;
pub use glam;

/// Must be implemented by all quantity types.
pub trait Quantity {
    fn f64(&self) -> f64;
}

pub mod prelude {
    pub use crate::{
        arcminutes, arcseconds, astronomical_units, degrees, degrees_per_second,
        degrees_per_second2, feet, feet2, feet_per_second, feet_per_second2, kelvin, kilograms,
        kilograms_meter2, kilograms_per_meter3, kilograms_per_second, kilometers, knots, meters,
        meters2, meters_per_second, meters_per_second2, miles, miles_per_hour, nautical_miles,
        nautical_miles_per_hour, newton_meters, newtons, pascals, pdl, pounds_force, pounds_mass,
        pounds_mass_per_second, pounds_per_feet3, pounds_square_foot, radians, radians_per_second,
        radians_per_second2, scalar,
        scalar::{
            acceleration::Acceleration,
            angle::{Angle, AngleUnit},
            angular_acceleration::AngularAcceleration,
            angular_velocity::AngularVelocity,
            area::Area,
            density::Density,
            dynamic_unit::DynamicUnits,
            force::{Force, ForceUnit},
            length::{Length, LengthUnit},
            mass::{Mass, MassUnit},
            mass_rate::MassRate,
            pressure::{Pressure, PressureUnit},
            rotational_inertia::RotationalInertia,
            temperature::{Temperature, TemperatureUnit},
            time::{Time, TimeUnit},
            torque::Torque,
            velocity::Velocity,
            velocity_squared::VelocitySquared,
            volume::Volume,
        },
        seconds,
        unit::{
            arcminutes::ArcMinutes, arcseconds::ArcSeconds, astronomical_units::AstronomicalUnits,
            celsius::Celsius, degrees::Degrees, fahrenheit::Fahrenheit, feet::Feet, hours::Hours,
            kelvin::Kelvin, kilograms::Kilograms, kilometers::Kilometers, meters::Meters,
            miles::Miles, nautical_miles::NauticalMiles, newtons::Newtons, pascals::Pascals,
            pounds_force::PoundsForce, pounds_mass::PoundsMass,
            pounds_square_foot::PoundsSquareFoot, radians::Radians, rankine::Rankine,
            scalar::Scalar, seconds::Seconds, slugs::Slugs, Unit,
        },
        vector::{pt3::Pt3, v3::V3},
        Quantity,
    };
}
pub use crate::prelude::*;

// For use from macros
pub use approx;
pub use num_traits;
pub use ordered_float;
