//! Small forward-mode automatic differentiation engine. Each jet carries six
//! independent derivatives, enough for the complete four-fin allocation.
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub v: f64,
    pub d: [f64; 6],
}
impl Jet {
    pub fn c(v: f64) -> Self {
        Self { v, d: [0.0; 6] }
    }
    pub fn var(v: f64, i: usize) -> Self {
        let mut x = Self::c(v);
        x.d[i] = 1.0;
        x
    }
    fn map(self, v: f64, slope: f64) -> Self {
        Self {
            v,
            d: self.d.map(|d| d * slope),
        }
    }
    pub fn sin(self) -> Self {
        self.map(self.v.sin(), self.v.cos())
    }
    pub fn cos(self) -> Self {
        self.map(self.v.cos(), -self.v.sin())
    }
    pub fn exp(self) -> Self {
        let v = self.v.exp();
        self.map(v, v)
    }
    pub fn ln(self) -> Self {
        self.map(self.v.ln(), 1.0 / self.v)
    }
    pub fn sqrt(self) -> Self {
        let v = self.v.sqrt();
        self.map(v, 0.5 / v)
    }
    pub fn acos(self) -> Self {
        self.map(self.v.acos(), -1.0 / (1.0 - self.v * self.v).sqrt())
    }
    pub fn tanh(self) -> Self {
        let v = self.v.tanh();
        self.map(v, 1.0 - v * v)
    }
    pub fn sq(self) -> Self {
        self * self
    }
    pub fn atan2(self, x: Self) -> Self {
        let denom = self.v * self.v + x.v * x.v;
        Self {
            v: self.v.atan2(x.v),
            d: std::array::from_fn(|i| (x.v * self.d[i] - self.v * x.d[i]) / denom),
        }
    }
}
impl Add for Jet {
    type Output = Self;
    fn add(self, r: Self) -> Self {
        Self {
            v: self.v + r.v,
            d: std::array::from_fn(|i| self.d[i] + r.d[i]),
        }
    }
}
impl Sub for Jet {
    type Output = Self;
    fn sub(self, r: Self) -> Self {
        Self {
            v: self.v - r.v,
            d: std::array::from_fn(|i| self.d[i] - r.d[i]),
        }
    }
}
impl Mul for Jet {
    type Output = Self;
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, r: Self) -> Self {
        Self {
            v: self.v * r.v,
            d: std::array::from_fn(|i| self.d[i] * r.v + self.v * r.d[i]),
        }
    }
}
impl Div for Jet {
    type Output = Self;
    // Quotient rule intentionally combines multiplication and subtraction.
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, r: Self) -> Self {
        Self {
            v: self.v / r.v,
            d: std::array::from_fn(|i| (self.d[i] * r.v - self.v * r.d[i]) / (r.v * r.v)),
        }
    }
}
impl Neg for Jet {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            v: -self.v,
            d: self.d.map(|v| -v),
        }
    }
}
impl Add<f64> for Jet {
    type Output = Self;
    fn add(self, r: f64) -> Self {
        self + Self::c(r)
    }
}
impl Sub<f64> for Jet {
    type Output = Self;
    fn sub(self, r: f64) -> Self {
        self - Self::c(r)
    }
}
impl Mul<f64> for Jet {
    type Output = Self;
    fn mul(self, r: f64) -> Self {
        self * Self::c(r)
    }
}
impl Div<f64> for Jet {
    type Output = Self;
    fn div(self, r: f64) -> Self {
        self / Self::c(r)
    }
}
