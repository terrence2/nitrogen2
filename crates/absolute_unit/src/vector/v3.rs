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
use crate::{Length, LengthUnit, Pt3, Quantity, Scalar};
use glam::{DQuat, DVec3};
use std::{
    fmt,
    marker::PhantomData,
    ops::{Add, AddAssign, Div, Mul, Sub},
};

/// A 64-bit 3-vec with <Quantity>.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3<T>
where
    T: Quantity + From<f64> + 'static,
{
    vec: DVec3,
    unit: PhantomData<T>,
}

impl<T> V3<T>
where
    T: Quantity + Clone + From<f64> + 'static,
{
    pub fn new(x: T, y: T, z: T) -> Self {
        Self {
            vec: DVec3::new(x.f64(), y.f64(), z.f64()),
            unit: PhantomData,
        }
    }

    pub fn new_quantity(x: f64, y: f64, z: f64) -> Self {
        Self {
            vec: DVec3::new(x, y, z),
            unit: PhantomData,
        }
    }

    pub fn new_dvec3(vec: DVec3) -> Self {
        Self {
            vec,
            unit: PhantomData,
        }
    }

    pub fn zero() -> Self {
        Self {
            vec: DVec3::ZERO,
            unit: PhantomData,
        }
    }

    pub fn dvec3(&self) -> &DVec3 {
        &self.vec
    }

    pub fn is_finite(&self) -> bool {
        self.vec.is_finite()
    }

    pub fn magnitude(&self) -> T {
        T::from(self.vec.length())
    }

    #[inline]
    pub fn x(&self) -> T {
        T::from(self.vec.x)
    }

    #[inline]
    pub fn y(&self) -> T {
        T::from(self.vec.y)
    }

    #[inline]
    pub fn z(&self) -> T {
        T::from(self.vec.z)
    }

    #[inline]
    pub fn set_x(&mut self, v: T) {
        self.vec.x = v.f64();
    }

    #[inline]
    pub fn set_y(&mut self, v: T) {
        self.vec.y = v.f64();
    }

    #[inline]
    pub fn set_z(&mut self, v: T) {
        self.vec.z = v.f64();
    }

    pub fn normalize(&self) -> DVec3 {
        self.vec.normalize()
    }

    pub fn normalize_or_zero(&self) -> DVec3 {
        self.vec.normalize_or_zero()
    }

    pub fn cross(&self, rhs: V3<T>) -> DVec3 {
        self.vec.cross(rhs.vec)
    }

    pub fn dot(&self, rhs: V3<T>) -> T {
        T::from(self.vec.dot(rhs.vec))
    }

    pub fn reflect(&self, normal: DVec3) -> Self {
        Self::new_dvec3(self.vec.reflect(normal))
    }
}

impl<U> V3<Length<U>>
where
    U: LengthUnit,
{
    pub fn pt3(&self) -> Pt3<U> {
        Pt3::new_dvec3(self.vec)
    }
}

impl<T> Mul<V3<T>> for DQuat
where
    T: Quantity + Clone + From<f64> + 'static,
{
    type Output = V3<T>;

    fn mul(self, rhs: V3<T>) -> Self::Output {
        V3::new_dvec3(self * rhs.vec)
    }
}

impl<T> Mul<V3<T>> for Scalar
where
    T: Quantity + Clone + From<f64> + 'static,
{
    type Output = V3<T>;

    fn mul(self, rhs: V3<T>) -> Self::Output {
        V3::new_dvec3(rhs.vec * self.f64())
    }
}

impl<T> Mul<Scalar> for V3<T>
where
    T: Quantity + Clone + From<f64> + 'static,
{
    type Output = V3<T>;

    fn mul(self, rhs: Scalar) -> Self::Output {
        V3::new_dvec3(self.vec * rhs.f64())
    }
}

impl<T> Add<V3<T>> for V3<T>
where
    T: Quantity + Clone + From<f64> + 'static,
{
    type Output = V3<T>;

    fn add(self, rhs: V3<T>) -> Self::Output {
        V3::new_dvec3(self.vec + rhs.vec)
    }
}

impl<T> AddAssign<V3<T>> for V3<T>
where
    T: Quantity + Clone + From<f64> + 'static,
{
    fn add_assign(&mut self, rhs: V3<T>) {
        self.vec += rhs.vec;
    }
}

impl<T> AddAssign<&V3<T>> for V3<T>
where
    T: Quantity + Clone + From<f64> + 'static,
{
    fn add_assign(&mut self, rhs: &V3<T>) {
        self.vec += rhs.vec;
    }
}

impl<T> Sub<V3<T>> for V3<T>
where
    T: Quantity + Clone + From<f64> + 'static,
{
    type Output = V3<T>;

    fn sub(self, rhs: V3<T>) -> Self::Output {
        V3::new_dvec3(self.vec - rhs.vec)
    }
}

impl<T> Div<Scalar> for V3<T>
where
    T: Quantity + Clone + From<f64> + 'static,
{
    type Output = V3<T>;

    fn div(self, rhs: Scalar) -> Self::Output {
        V3::new_dvec3(self.vec / rhs.f64())
    }
}

impl<T> fmt::Display for V3<T>
where
    T: Quantity + Clone + From<f64> + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: need a way to show the unit suffix
        // write!(f, "{}", T::UNIT_SHORT_NAME)
        fmt::Display::fmt(&self.vec, f)
    }
}
