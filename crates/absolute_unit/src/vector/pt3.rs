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
use crate::{Length, LengthUnit, Meters, Quantity, Scalar, V3, meters};
use approx::{AbsDiffEq, RelativeEq, abs_diff_eq, relative_eq};
use glam::{DMat4, DQuat, DVec3, DVec4};
use nalgebra as na;
use std::{
    fmt,
    marker::PhantomData,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub},
};

/// A 64-bit 3-vec with length in <LengthUnit>s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt3<Unit>
where
    Unit: LengthUnit,
{
    vec: DVec3,
    unit: PhantomData<Unit>,
}

impl<Unit> Pt3<Unit>
where
    Unit: LengthUnit,
{
    #[inline]
    pub fn new(x: Length<Unit>, y: Length<Unit>, z: Length<Unit>) -> Self {
        Self {
            vec: DVec3::new(x.f64(), y.f64(), z.f64()),
            unit: PhantomData,
        }
    }

    #[inline]
    pub fn new_unit(x: f64, y: f64, z: f64) -> Self {
        Self {
            vec: DVec3::new(x, y, z),
            unit: PhantomData,
        }
    }

    #[inline]
    pub fn new_dvec3(vec: DVec3) -> Self {
        Self {
            vec,
            unit: PhantomData,
        }
    }

    #[inline]
    pub fn zero() -> Self {
        Self {
            vec: DVec3::ZERO,
            unit: PhantomData,
        }
    }

    #[inline]
    pub fn infinity() -> Self {
        Self {
            vec: DVec3::INFINITY,
            unit: PhantomData,
        }
    }

    #[inline]
    pub fn neg_infinity() -> Self {
        Self {
            vec: DVec3::NEG_INFINITY,
            unit: PhantomData,
        }
    }

    #[inline]
    pub fn is_finite(&self) -> bool {
        self.vec.is_finite()
    }

    #[inline]
    pub fn to_array(&self) -> [Length<Unit>; 3] {
        [self.x(), self.y(), self.z()]
    }

    #[inline]
    pub fn x(&self) -> Length<Unit> {
        Length::<Unit>::from(&self.vec.x)
    }

    #[inline]
    pub fn y(&self) -> Length<Unit> {
        Length::<Unit>::from(&self.vec.y)
    }

    #[inline]
    pub fn z(&self) -> Length<Unit> {
        Length::<Unit>::from(&self.vec.z)
    }

    #[inline]
    pub fn set_x<T: LengthUnit>(&mut self, value: Length<T>) {
        self.vec.x = Length::<Unit>::from(&value).f64();
    }

    #[inline]
    pub fn set_y<T: LengthUnit>(&mut self, value: Length<T>) {
        self.vec.y = Length::<Unit>::from(&value).f64();
    }

    #[inline]
    pub fn set_z<T: LengthUnit>(&mut self, value: Length<T>) {
        self.vec.z = Length::<Unit>::from(&value).f64();
    }

    #[inline]
    pub fn with_x<T: LengthUnit>(&self, value: Length<T>) -> Self {
        Self::new(Length::<Unit>::from(&value), self.y(), self.z())
    }

    #[inline]
    pub fn with_y<T: LengthUnit>(&self, value: Length<T>) -> Self {
        Self::new(self.x(), Length::<Unit>::from(&value), self.z())
    }

    #[inline]
    pub fn with_z<T: LengthUnit>(&self, value: Length<T>) -> Self {
        Self::new(self.x(), self.y(), Length::<Unit>::from(&value))
    }

    #[inline]
    pub fn x_as<T: LengthUnit>(&self) -> Length<T> {
        Length::<T>::from(&Length::<Unit>::from(&self.vec.x))
    }

    #[inline]
    pub fn y_as<T: LengthUnit>(&self) -> Length<T> {
        Length::<T>::from(&Length::<Unit>::from(&self.vec.y))
    }

    #[inline]
    pub fn z_as<T: LengthUnit>(&self) -> Length<T> {
        Length::<T>::from(&Length::<Unit>::from(&self.vec.z))
    }

    pub fn at(&self, index: usize) -> Length<Unit> {
        match index {
            0 => Length::<Unit>::from(self.vec.x),
            1 => Length::<Unit>::from(self.vec.y),
            2 => Length::<Unit>::from(self.vec.z),
            _ => panic!("invalid pt3 index at: {index}"),
        }
    }

    pub fn set(&mut self, index: usize, value: Length<Unit>) {
        match index {
            0 => self.vec.x = value.f64(),
            1 => self.vec.y = value.f64(),
            2 => self.vec.z = value.f64(),
            _ => panic!("invalid pt3 index set: {index}"),
        }
    }

    pub fn transform_by(&self, m: &DMat4) -> Self {
        Self::new_dvec3(m.transform_point3(self.vec))
    }

    #[inline]
    pub fn length(&self) -> Length<Unit> {
        Length::<Unit>::from(self.vec.length())
    }

    #[inline]
    pub fn cross(&self, rhs: Pt3<Unit>) -> V3<Length<Unit>> {
        V3::new_dvec3(self.vec.cross(rhs.vec))
    }

    #[inline]
    pub fn to(&self, other: Pt3<Unit>) -> V3<Length<Unit>> {
        V3::new_dvec3(other.vec - self.vec)
    }

    #[inline]
    pub fn v3(&self) -> V3<Length<Unit>> {
        V3::new_dvec3(self.vec)
    }

    #[inline]
    pub fn dvec3(&self) -> DVec3 {
        self.vec
    }

    #[inline]
    pub fn dvec4(&self, w: f64) -> DVec4 {
        DVec4::from((self.vec, w))
    }

    #[inline]
    pub fn na_dvec3(&self) -> na::Vector3<f64> {
        na::Vector3::new(self.vec.x, self.vec.y, self.vec.z)
    }
}

impl<Unit> AbsDiffEq for Pt3<Unit>
where
    Unit: LengthUnit,
{
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        abs_diff_eq!(self.vec, other.vec, epsilon = epsilon)
    }
}

impl<Unit> RelativeEq for Pt3<Unit>
where
    Unit: LengthUnit,
{
    fn default_max_relative() -> Self::Epsilon {
        f64::default_epsilon()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        relative_eq!(
            self.vec,
            other.vec,
            epsilon = epsilon,
            max_relative = max_relative
        )
    }
}

impl<A, B> Add<Pt3<B>> for Pt3<A>
where
    A: LengthUnit,
    B: LengthUnit,
{
    type Output = Pt3<A>;

    fn add(self, rhs: Pt3<B>) -> Self::Output {
        let rhs = Pt3::<A>::from(&rhs);
        Pt3::new_dvec3(self.vec + rhs.vec)
    }
}

impl<A, B> AddAssign<Pt3<B>> for Pt3<A>
where
    A: LengthUnit,
    B: LengthUnit,
{
    fn add_assign(&mut self, rhs: Pt3<B>) {
        let rhs = Pt3::<A>::from(&rhs);
        self.vec += rhs.vec;
    }
}

impl<A, B> Add<V3<Length<B>>> for Pt3<A>
where
    A: LengthUnit,
    B: LengthUnit,
{
    type Output = Pt3<A>;

    fn add(self, rhs: V3<Length<B>>) -> Self::Output {
        let rhs = Pt3::<B>::new(rhs.x(), rhs.y(), rhs.z());
        let rhs = Pt3::<A>::from(&rhs);
        Pt3::new_dvec3(self.vec + rhs.vec)
    }
}

impl<A, B> Sub<Pt3<B>> for Pt3<A>
where
    A: LengthUnit,
    B: LengthUnit,
{
    type Output = Pt3<A>;

    fn sub(self, rhs: Pt3<B>) -> Self::Output {
        let rhs = Pt3::<A>::from(&rhs);
        Pt3::new_dvec3(self.vec - rhs.vec)
    }
}

impl<Unit> Mul<Pt3<Unit>> for DQuat
where
    Unit: LengthUnit,
{
    type Output = Pt3<Unit>;

    fn mul(self, rhs: Pt3<Unit>) -> Self::Output {
        Pt3::new_dvec3(self * rhs.vec)
    }
}

impl<Unit> Mul<Pt3<Unit>> for DMat4
where
    Unit: LengthUnit,
{
    type Output = Pt3<Unit>;

    fn mul(self, rhs: Pt3<Unit>) -> Self::Output {
        Pt3::new_dvec3(self.transform_point3(rhs.vec))
    }
}

impl<Unit> Mul<Pt3<Unit>> for Scalar
where
    Unit: LengthUnit,
{
    type Output = Pt3<Unit>;

    fn mul(self, rhs: Pt3<Unit>) -> Self::Output {
        Pt3::new_dvec3(self.f64() * rhs.vec)
    }
}

impl<Unit> Neg for Pt3<Unit>
where
    Unit: LengthUnit,
{
    type Output = Pt3<Unit>;

    fn neg(self) -> Self::Output {
        Pt3::new_dvec3(-self.vec)
    }
}

impl<Unit> Div<Scalar> for Pt3<Unit>
where
    Unit: LengthUnit,
{
    type Output = Pt3<Unit>;

    fn div(self, rhs: Scalar) -> Self::Output {
        Pt3::new_dvec3(self.vec / rhs.0.0)
    }
}

impl<Unit> fmt::Display for Pt3<Unit>
where
    Unit: LengthUnit,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        fmt::Display::fmt(&self.vec.x, f)?;
        write!(f, ", ")?;
        fmt::Display::fmt(&self.vec.y, f)?;
        write!(f, ", ")?;
        fmt::Display::fmt(&self.vec.z, f)?;
        write!(f, "]{}", Unit::UNIT_SHORT_NAME)
    }
}

impl<'a, UnitA, UnitB> From<&'a Pt3<UnitB>> for Pt3<UnitA>
where
    UnitA: LengthUnit,
    UnitB: LengthUnit,
{
    fn from(value: &'a Pt3<UnitB>) -> Self {
        Self::new(
            value.x_as::<UnitA>(),
            value.y_as::<UnitA>(),
            value.z_as::<UnitA>(),
        )
    }
}

impl From<&na::Point3<f64>> for Pt3<Meters> {
    fn from(value: &na::Point3<f64>) -> Self {
        Self::new(meters!(value.x), meters!(value.y), meters!(value.z))
    }
}

impl From<na::Point3<f64>> for Pt3<Meters> {
    fn from(value: na::Point3<f64>) -> Self {
        Self::new(meters!(value.x), meters!(value.y), meters!(value.z))
    }
}
