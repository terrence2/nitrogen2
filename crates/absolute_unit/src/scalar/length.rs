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
use crate::{
    impl_value_type_conversions, radians, scalar, supports_absdiffeq, supports_cancellation,
    supports_quantity_ops, supports_scalar_ops, supports_shift_ops, supports_value_type_conversion,
    Angle, Area, DynamicUnits, Radians, Scalar, Unit,
};
use ordered_float::OrderedFloat;
use std::{fmt, fmt::Debug, marker::PhantomData, ops::Mul};

pub trait LengthUnit: Unit + Copy + Debug + Eq + PartialEq + Ord + PartialOrd + 'static {
    const METERS_IN_UNIT: f64;
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Length<Unit: LengthUnit> {
    v: OrderedFloat<f64>, // in Unit
    phantom_1: PhantomData<Unit>,
}
supports_quantity_ops!(Length<A>, LengthUnit);
supports_shift_ops!(Length<A1>, Length<A2>, LengthUnit);
supports_scalar_ops!(Length<A>, LengthUnit);
supports_cancellation!(Length<A1>, Length<A2>, LengthUnit);
supports_absdiffeq!(Length<A>, LengthUnit);
supports_value_type_conversion!(Length<A>, LengthUnit, impl_value_type_conversions);

impl<L> Length<L>
where
    L: LengthUnit,
{
    pub const fn new(v: f64) -> Self {
        Self {
            v: OrderedFloat(v),
            phantom_1: PhantomData,
        }
    }

    pub fn as_dyn(&self) -> DynamicUnits {
        DynamicUnits::new1o0::<L>(self.v)
    }

    pub fn atan2(&self, other: &Length<L>) -> Angle<Radians> {
        radians!(self.v.atan2(other.v.0))
    }

    pub fn abs(&self) -> Length<L> {
        self.v.abs().into()
    }

    pub fn signum(&self) -> Scalar {
        scalar!(self.v.signum())
    }
}

impl<Unit> fmt::Display for Length<Unit>
where
    Unit: LengthUnit,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.v.0, f)?;
        write!(f, "{}", Unit::UNIT_SUFFIX)
    }
}

impl<'a, UnitA, UnitB> From<&'a Length<UnitA>> for Length<UnitB>
where
    UnitA: LengthUnit,
    UnitB: LengthUnit,
{
    fn from(v: &'a Length<UnitA>) -> Self {
        Self {
            v: v.v * UnitA::METERS_IN_UNIT / UnitB::METERS_IN_UNIT,
            phantom_1: PhantomData,
        }
    }
}

impl<UnitA, UnitB> Mul<Length<UnitB>> for Length<UnitA>
where
    UnitA: LengthUnit,
    UnitB: LengthUnit,
{
    type Output = Area<UnitA>;

    fn mul(self, other: Length<UnitB>) -> Self::Output {
        Area::<UnitA>::from(self.v.0 * Length::<UnitA>::from(&other).f64())
    }
}

#[cfg(test)]
mod test {
    use crate::{feet, kilometers, meters, scalar};
    use approx::assert_abs_diff_eq;

    #[test]
    fn test_meters_to_feet() {
        let m = meters!(1);
        println!("m : {m}");
        println!("ft: {}", feet!(m));
        assert_abs_diff_eq!(kilometers!(m), kilometers!(0.001));
    }

    #[test]
    fn test_scalar_length() {
        assert_abs_diff_eq!(meters!(2) * scalar!(2), meters!(4));
    }
}
