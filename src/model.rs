//! Paper equations with explicitly illustrative acoustics and geometry.
use crate::ad::Jet;
use nalgebra::{SMatrix, SVector};
use serde::{Deserialize, Serialize};
use std::f64::consts::{LN_10, PI};

pub const AMIN: f64 = 0.4;
pub const AMAX: f64 = 1.2;
pub const FMIN: f64 = 0.3;
pub const FMAX: f64 = 2.3;
pub const K0: f64 = 1.0; // Illustrative N/Hz^2, not calibrated from the paper.

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Profile {
    Catfish,
    Salmon,
    Broadband,
}
impl Profile {
    pub const ALL: [Self; 3] = [Self::Catfish, Self::Salmon, Self::Broadband];
    pub fn label(self) -> &'static str {
        match self {
            Self::Catfish => "Catfish-like",
            Self::Salmon => "Salmon-like",
            Self::Broadband => "Broadband",
        }
    }
    pub fn knots(self) -> &'static [(f64, f64)] {
        match self {
            Self::Catfish => &[
                (50., 35.),
                (100., 20.),
                (200., 10.),
                (500., 0.),
                (1000., 4.),
                (2000., 14.),
                (4000., 32.),
            ],
            Self::Salmon => &[
                (30., 24.),
                (50., 12.),
                (100., 0.),
                (200., 5.),
                (400., 22.),
                (700., 40.),
            ],
            Self::Broadband => &[(10., 0.), (22050., 0.)],
        }
    }
    pub fn threshold(self, hz: f64) -> Option<f64> {
        let k = self.knots();
        if hz < k[0].0 || hz > k[k.len() - 1].0 {
            return None;
        }
        for w in k.windows(2) {
            if hz <= w[1].0 {
                let t = (hz / w[0].0).ln() / (w[1].0 / w[0].0).ln();
                return Some(w[0].1 + t * (w[1].1 - w[0].1));
            }
        }
        Some(k[k.len() - 1].1)
    }
    pub fn weight(self, hz: f64) -> f64 {
        self.threshold(hz).map_or(0., |t| 10f64.powf(-t / 10.))
    }
}

#[derive(Clone)]
pub struct Acoustics {
    pub profile: Profile,
    pub coeff: [f64; 4],
}
impl Acoustics {
    pub fn new(profile: Profile) -> Self {
        let mut coeff = [0.; 4];
        // 44.1 kHz / 4096 bins; integrate in linear power. Synthetic PSD is a
        // sum of fixed spectral bases, so this factorization has exact gradients.
        let df = 44100. / 4096.;
        for k in 1..=2048 {
            let hz = k as f64 * df;
            let b = spectral_basis(hz);
            for j in 0..4 {
                coeff[j] += b[j] * profile.weight(hz) * df;
            }
        }
        Self { profile, coeff }
    }
    pub fn components(a: Jet, f: Jet, t: Jet) -> [Jet; 4] {
        let lo = ((a - 0.90).sq() * 3.0 + (f - 1.65).sq() * 1.5 + (t.sin()).sq() * 0.1 - 11.).exp();
        let mid = ((a - 1.05).sq() * 4.0
            + (f - 0.75).sq() * 1.8
            + ((f * 5. + a * 3.).sin() + 1.) * 0.35
            + (t.cos() + 1.) * 0.08
            - 10.5)
            .exp();
        let hi = ((a - 0.48).sq() * 3.0 + (f - 1.25).sq() * 2.0 - 11.).exp();
        [lo, mid, hi, Jet::c(1e-9)]
    }
    pub fn score(&self, a: Jet, f: Jet, t: Jet) -> Jet {
        let c = Self::components(a, f, t);
        let p = (0..4).fold(Jet::c(1e-18), |s, i| s + c[i] * self.coeff[i]);
        p.ln() * (10. / LN_10)
    }
    pub fn value(&self, a: f64, f: f64, t: f64) -> f64 {
        self.score(Jet::c(a), Jet::c(f), Jet::c(t)).v
    }
    pub fn psd(&self, hz: f64, a: f64, f: f64, t: f64) -> f64 {
        let c = Self::components(Jet::c(a), Jet::c(f), Jet::c(t));
        let b = spectral_basis(hz);
        (0..4).map(|i| c[i].v * b[i]).sum()
    }
}
fn spectral_basis(hz: f64) -> [f64; 4] {
    let g = |mu: f64, s: f64| {
        (-0.5 * ((hz.ln() - mu.ln()) / s).powi(2)).exp() / (hz * s * (2. * PI).sqrt())
    };
    [g(110., 0.35), g(850., 0.42), g(4200., 0.45), 1. / 22050.]
}
pub fn force(a: f64, f: f64) -> f64 {
    K0 * f * f * (1. - a.cos())
}
pub fn force_jet(a: Jet, f: Jet) -> Jet {
    f.sq() * (Jet::c(1.) - a.cos()) * K0
}
pub fn interval(force: f64) -> Option<(f64, f64)> {
    if !force.is_finite() || force < 0. {
        return None;
    }
    if force < 1e-10 {
        return Some((FMIN, FMAX));
    }
    let lo = FMIN.max((force / (K0 * (1. - AMAX.cos()))).sqrt());
    let hi = FMAX.min((force / (K0 * (1. - AMIN.cos()))).sqrt());
    if lo <= hi { Some((lo, hi)) } else { None }
}
pub fn inverse(force: f64, f: f64) -> Option<f64> {
    if (0. ..1e-10).contains(&force) {
        return Some(0.);
    }
    let (lo, hi) = interval(force)?;
    if f < lo - 1e-9 || f > hi + 1e-9 {
        return None;
    }
    Some((1. - force / (K0 * f * f)).clamp(-1., 1.).acos())
}
pub fn power_sum(levels: &[f64]) -> Option<f64> {
    if levels.is_empty() {
        return None;
    }
    let m = levels.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Some(
        m + 10.
            * levels
                .iter()
                .map(|v| 10f64.powf((v - m) / 10.))
                .sum::<f64>()
                .log10(),
    )
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SingleSettings {
    pub target: f64,
    pub acoustic: f64,
    pub penalty: f64,
    pub regularization: f64,
    pub theta: f64,
    pub start: [f64; 2],
}
impl Default for SingleSettings {
    fn default() -> Self {
        Self {
            target: 0.65,
            acoustic: 1.,
            penalty: 45.,
            regularization: 0.15,
            theta: 0.,
            start: [0.68, 1.9],
        }
    }
}
#[derive(Clone, Copy, Serialize)]
pub struct Step {
    pub a: f64,
    pub f: f64,
    pub cost: f64,
    pub score: f64,
    pub force: f64,
    pub grad: [f64; 2],
}
pub fn single_cost(m: &Acoustics, s: &SingleSettings, a: Jet, f: Jet) -> Jet {
    m.score(a, f, Jet::c(s.theta)) * s.acoustic
        + (force_jet(a, f) - s.target).sq() * s.penalty
        + ((a - s.start[0]).sq() / ((AMAX - AMIN).powi(2))
            + (f - s.start[1]).sq() / ((FMAX - FMIN).powi(2)))
            * s.regularization
}
pub fn single_step(m: &Acoustics, s: &SingleSettings, a: f64, f: f64) -> Step {
    let j = single_cost(m, s, Jet::var(a, 0), Jet::var(f, 1));
    Step {
        a,
        f,
        cost: j.v,
        score: m.value(a, f, s.theta),
        force: force(a, f),
        grad: [j.d[0], j.d[1]],
    }
}
pub fn optimize_single(m: &Acoustics, s: &SingleSettings) -> Vec<Step> {
    if s.target <= 1e-10 {
        return vec![single_step(m, s, 0., FMIN)];
    }
    let mut p = single_step(m, s, s.start[0], s.start[1]);
    let mut path = vec![p];
    let mut mom = [0.; 2];
    let mut vel = [0.; 2];
    for k in 1..=100 {
        let mut dir = [0.; 2];
        for j in 0..2 {
            mom[j] = 0.9 * mom[j] + 0.1 * p.grad[j];
            vel[j] = 0.999 * vel[j] + 0.001 * p.grad[j] * p.grad[j];
            dir[j] = (mom[j] / (1. - 0.9f64.powi(k)))
                / ((vel[j] / (1. - 0.999f64.powi(k))).sqrt() + 1e-8);
        }
        // Armijo-style monotone backtracking; if momentum points uphill, use
        // normalized steepest descent. This is a teaching solver, not the paper's runtime.
        if dir[0] * p.grad[0] + dir[1] * p.grad[1] <= 0. {
            let n = p.grad[0].hypot(p.grad[1]).max(1e-12);
            dir = [p.grad[0] / n, p.grad[1] / n];
        }
        let mut accepted = None;
        let mut rate = 0.055;
        for _ in 0..18 {
            let c = single_step(
                m,
                s,
                (p.a - rate * dir[0]).clamp(AMIN, AMAX),
                (p.f - rate * dir[1]).clamp(FMIN, FMAX),
            );
            if c.cost < p.cost - 1e-11 {
                accepted = Some(c);
                break;
            }
            rate *= 0.5;
        }
        if let Some(c) = accepted {
            p = c;
            path.push(p);
        } else {
            break;
        }
    }
    path
}

pub type BMatrix = SMatrix<f64, 6, 8>;
pub type NMatrix = SMatrix<f64, 8, 2>;
pub type Q = SVector<f64, 8>;
#[derive(Clone)]
pub struct Geometry {
    pub b: BMatrix,
    pub n: NMatrix,
    pub positions: [[f64; 3]; 4],
    pub directions: [[f64; 3]; 4],
}
impl Geometry {
    pub fn demo() -> Self {
        let positions = [
            [0.65, 0.4, 0.],
            [0.65, -0.4, 0.],
            [-0.65, 0.4, 0.],
            [-0.65, -0.4, 0.],
        ];
        let c = 0.5f64.sqrt();
        let directions = [[c, c, 0.], [c, -c, 0.], [c, -c, 0.], [c, c, 0.]];
        let mut b = BMatrix::zeros();
        for i in 0..4 {
            for (col, d) in [(i, directions[i]), (i + 4, [0., 0., 1.])] {
                let p = positions[i];
                let moment = [
                    p[1] * d[2] - p[2] * d[1],
                    p[2] * d[0] - p[0] * d[2],
                    p[0] * d[1] - p[1] * d[0],
                ];
                for j in 0..3 {
                    b[(j, col)] = d[j];
                    b[(j + 3, col)] = moment[j];
                }
            }
        }
        // Symmetric illustrative geometry: exact orthonormal null-space basis.
        let mut n = NMatrix::zeros();
        for (i, v) in [0.5, 0.5, -0.5, -0.5].into_iter().enumerate() {
            n[(i, 0)] = v;
        }
        for (i, v) in [0.5, -0.5, -0.5, 0.5].into_iter().enumerate() {
            n[(i + 4, 1)] = v;
        }
        Self {
            b,
            n,
            positions,
            directions,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct AllocSettings {
    pub z: [f64; 2],
    pub frequencies: [f64; 4],
    pub heave: f64,
    pub surge: f64,
    pub deadband: f64,
    pub q_weight: f64,
    pub a_weight: f64,
    pub f_weight: f64,
}
impl Default for AllocSettings {
    fn default() -> Self {
        Self {
            z: [0.; 2],
            frequencies: [1.5; 4],
            heave: 0.28,
            surge: 0.55,
            deadband: 0.,
            q_weight: 0.05,
            a_weight: 0.02,
            f_weight: 0.02,
        }
    }
}
impl AllocSettings {
    pub fn qref(&self) -> Q {
        Q::from_row_slice(&[
            self.surge * 1.25,
            self.surge * 0.7,
            self.surge * 1.05,
            self.surge,
            self.heave,
            self.heave * 0.8,
            self.heave * 1.2,
            self.heave,
        ])
    }
}
#[derive(Clone)]
pub struct Allocation {
    pub q: Q,
    pub amplitude: [f64; 4],
    pub frequency: [f64; 4],
    pub theta: [f64; 4],
    pub scores: [f64; 4],
    pub active: [bool; 4],
    pub level: Option<f64>,
    pub cost: f64,
    pub residual: f64,
    pub feasible: bool,
}
pub fn allocation(
    m: &Acoustics,
    g: &Geometry,
    s: &AllocSettings,
    z: [f64; 2],
    f: [f64; 4],
) -> Allocation {
    let qr = s.qref();
    let q = qr + g.n * SVector::<f64, 2>::from_row_slice(&z);
    let mut a = [0.; 4];
    let mut theta = [0.; 4];
    let mut scores = [0.; 4];
    let mut active = [false; 4];
    let mut realized = Q::zeros();
    let mut feasible = true;
    for i in 0..4 {
        let mag = q[i].hypot(q[i + 4]);
        theta[i] = q[i + 4].atan2(q[i]);
        match inverse(mag, f[i]) {
            Some(v) => a[i] = v,
            None => {
                feasible = false;
                a[i] = 0.;
            }
        }
        if a[i] < s.deadband {
            a[i] = 0.;
        }
        active[i] = a[i] > 1e-10;
        if active[i] {
            scores[i] = m.value(a[i], f[i], theta[i]);
        }
        let realized_mag = force(a[i], f[i]);
        realized[i] = realized_mag * theta[i].cos();
        realized[i + 4] = realized_mag * theta[i].sin();
    }
    let level = power_sum(
        &(0..4)
            .filter(|i| active[*i])
            .map(|i| scores[i])
            .collect::<Vec<_>>(),
    );
    let residual = (g.b * realized - g.b * qr).norm();
    let qscale = (qr.iter().map(|v| v.abs()).sum::<f64>() / 8.).max(0.05);
    let mut cost = level.map_or(0., |v| (v + 40.) / 10.); // illustrative surrogate target mean/std
    cost += s.q_weight / 8. * ((q - qr) / qscale).norm_squared();
    for i in 0..4 {
        let ar = inverse(qr[i].hypot(qr[i + 4]), 1.5).unwrap_or(0.);
        cost += s.a_weight / 4. * ((a[i] - ar) / (AMAX - AMIN)).powi(2)
            + s.f_weight / 4. * ((f[i] - 1.5) / (FMAX - FMIN)).powi(2);
    }
    Allocation {
        q,
        amplitude: a,
        frequency: f,
        theta,
        scores,
        active,
        level,
        cost,
        residual,
        feasible: feasible && residual < 1e-8,
    }
}

// Bound coordinates smoothly. Frequency endpoints depend on the current force,
// so their derivatives participate in the six-variable gradient.
type Decoded = ([Jet; 2], [Jet; 4], [Jet; 4], [Jet; 4], [Jet; 8]);
pub fn decode(g: &Geometry, s: &AllocSettings, x: [Jet; 6]) -> Option<Decoded> {
    let z = [x[0].tanh() * 1.1, x[1].tanh() * 1.1];
    let qr = s.qref();
    let q: [Jet; 8] =
        std::array::from_fn(|i| Jet::c(qr[i]) + z[0] * g.n[(i, 0)] + z[1] * g.n[(i, 1)]);
    let mut a = [Jet::c(0.); 4];
    let mut f = [Jet::c(FMIN); 4];
    let mut theta = [Jet::c(0.); 4];
    for i in 0..4 {
        if q[i].v.hypot(q[i + 4].v) < 1e-10 {
            continue;
        }
        let mag = (q[i].sq() + q[i + 4].sq()).sqrt();
        let low = (mag / (K0 * (1. - AMAX.cos()))).sqrt();
        let high = (mag / (K0 * (1. - AMIN.cos()))).sqrt();
        let lo = if low.v > FMIN { low } else { Jet::c(FMIN) };
        let hi = if high.v < FMAX { high } else { Jet::c(FMAX) };
        if lo.v > hi.v {
            return None;
        }
        f[i] = lo + (hi - lo) * ((x[i + 2].tanh() + 1.) * 0.5);
        a[i] = (Jet::c(1.) - mag / (f[i].sq() * K0)).acos();
        theta[i] = q[i + 4].atan2(q[i]);
    }
    Some((z, f, a, theta, q))
}
pub fn allocation_cost(m: &Acoustics, g: &Geometry, s: &AllocSettings, x: [Jet; 6]) -> Option<Jet> {
    let (_, f, a, t, q) = decode(g, s, x)?;
    let qr = s.qref();
    let levels: Vec<Jet> = (0..4)
        .filter(|i| a[*i].v > 1e-10)
        .map(|i| m.score(a[i], f[i], t[i]))
        .collect();
    let peak = levels.iter().map(|v| v.v).fold(f64::NEG_INFINITY, f64::max);
    let mut cost = if levels.is_empty() {
        Jet::c(0.)
    } else {
        (levels
            .iter()
            .fold(Jet::c(0.), |sum, v| {
                sum + ((*v - peak) * (LN_10 / 10.)).exp()
            })
            .ln()
            * (10. / LN_10)
            + peak
            + 40.)
            / 10.
    };
    let scale = (qr.iter().map(|v| v.abs()).sum::<f64>() / 8.).max(0.05);
    for i in 0..8 {
        cost = cost + ((q[i] - qr[i]) / scale).sq() * (s.q_weight / 8.);
    }
    for i in 0..4 {
        let ar = inverse(qr[i].hypot(qr[i + 4]), 1.5).unwrap_or(0.);
        cost = cost
            + ((a[i] - ar) / (AMAX - AMIN)).sq() * (s.a_weight / 4.)
            + ((f[i] - 1.5) / (FMAX - FMIN)).sq() * (s.f_weight / 4.);
    }
    Some(cost)
}
pub fn refine(m: &Acoustics, g: &Geometry, s: &AllocSettings) -> (AllocSettings, String) {
    let baseline = allocation(m, g, s, [0.; 2], [1.5; 4]);
    if !baseline.feasible {
        return (
            s.clone(),
            "Nominal command fails feasibility / post-processing; adjust request or deadband."
                .into(),
        );
    }
    let mut seeds = vec![
        [0., 0.],
        [0.35, 0.],
        [-0.35, 0.],
        [0., 0.35],
        [0., -0.35],
        s.z,
    ];
    seeds.dedup();
    let mut best = baseline.cost;
    let mut result = s.clone();
    result.z = [0.; 2];
    result.frequencies = [1.5; 4];
    for z in seeds {
        for fseed in [1.0, 1.5, 2.0] {
            let q = s.qref() + g.n * SVector::<f64, 2>::from_row_slice(&z);
            let mut x = [0.; 6];
            x[0] = (z[0] / 1.1).clamp(-0.99, 0.99).atanh();
            x[1] = (z[1] / 1.1).clamp(-0.99, 0.99).atanh();
            let mut valid = true;
            for i in 0..4 {
                if let Some((lo, hi)) = interval(q[i].hypot(q[i + 4])) {
                    let f = if z == s.z { s.frequencies[i] } else { fseed };
                    let t = ((f - lo) / (hi - lo).max(1e-10)).clamp(0.001, 0.999);
                    x[i + 2] = (2. * t - 1.).atanh();
                } else {
                    valid = false;
                }
            }
            if !valid {
                continue;
            }
            let mut mom = [0.; 6];
            let mut vel = [0.; 6];
            for k in 0..=4 {
                // paper's four Adam refinement steps, deterministic pre-screen
                let Some(j) = allocation_cost(m, g, s, std::array::from_fn(|i| Jet::var(x[i], i)))
                else {
                    break;
                };
                if !j.v.is_finite() || j.d.iter().any(|v| !v.is_finite()) {
                    break;
                }
                if let Some((zz, ff, _, _, _)) = decode(g, s, x.map(Jet::c)) {
                    let c = allocation(m, g, s, zz.map(|v| v.v), ff.map(|v| v.v));
                    if c.feasible && c.cost < best - 1e-10 {
                        best = c.cost;
                        result.z = zz.map(|v| v.v);
                        result.frequencies = ff.map(|v| v.v);
                    }
                }
                if k == 4 {
                    break;
                }
                for i in 0..6 {
                    mom[i] = 0.9 * mom[i] + 0.1 * j.d[i];
                    vel[i] = 0.999 * vel[i] + 0.001 * j.d[i] * j.d[i];
                    x[i] -= 0.06 * (mom[i] / (1. - 0.9f64.powi(k + 1)))
                        / ((vel[i] / (1. - 0.999f64.powi(k + 1))).sqrt() + 1e-8);
                }
            }
        }
    }
    let status = if best < baseline.cost - 1e-10 {
        format!(
            "Accepted: J4 {:.4} -> {:.4}; final commands preserve model wrench.",
            baseline.cost, best
        )
    } else {
        "Fallback: no feasible improving candidate; nominal command retained.".into()
    };
    (result, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn force_inverse_roundtrip() {
        for a in [0.4, 0.7, 1.2] {
            for f in [0.3, 1.2, 2.3] {
                let mag = force(a, f);
                assert!((inverse(mag, f).unwrap() - a).abs() < 1e-10);
            }
        }
        assert!(interval(100.).is_none());
        assert_eq!(inverse(0., 1.), Some(0.));
    }
    #[test]
    fn nullspace_preserves_full_wrench() {
        let g = Geometry::demo();
        assert!((g.b * g.n).norm() < 1e-12);
        assert!((g.n.transpose() * g.n - SMatrix::<f64, 2, 2>::identity()).norm() < 1e-12);
        assert_eq!(g.b.rank(1e-9), 6);
        let s = AllocSettings::default();
        let q = s.qref() + g.n * SVector::<f64, 2>::new(0.8, -0.6);
        assert!((g.b * q - g.b * s.qref()).norm() < 1e-12);
    }
    #[test]
    fn equal_incoherent_sources_add_six_db() {
        assert!((power_sum(&[-40.; 4]).unwrap() + 40. - 10. * 4f64.log10()).abs() < 1e-12);
        assert!(power_sum(&[]).is_none());
    }
    #[test]
    fn auditory_support_and_log_interpolation() {
        let p = Profile::Catfish;
        assert_eq!(p.weight(20.), 0.);
        assert_eq!(p.weight(500.), 1.);
        assert!((p.threshold((100f64 * 200.).sqrt()).unwrap() - 15.).abs() < 1e-10);
    }
    #[test]
    fn single_gradient_matches_finite_difference() {
        let m = Acoustics::new(Profile::Catfish);
        let s = SingleSettings::default();
        let x = [0.81, 1.53];
        let ad = single_cost(&m, &s, Jet::var(x[0], 0), Jet::var(x[1], 1));
        for i in 0..2 {
            let mut xp = x;
            let mut xm = x;
            xp[i] += 1e-6;
            xm[i] -= 1e-6;
            let fd = (single_cost(&m, &s, Jet::c(xp[0]), Jet::c(xp[1])).v
                - single_cost(&m, &s, Jet::c(xm[0]), Jet::c(xm[1])).v)
                / 2e-6;
            assert!((ad.d[i] - fd).abs() < 1e-5);
        }
    }
    #[test]
    fn six_variable_gradient_matches_finite_difference() {
        let m = Acoustics::new(Profile::Catfish);
        let g = Geometry::demo();
        let s = AllocSettings::default();
        let x = [0.12, -0.2, 0.2, -0.4, 0.1, -0.1];
        let ad = allocation_cost(&m, &g, &s, std::array::from_fn(|i| Jet::var(x[i], i))).unwrap();
        for i in 0..6 {
            let mut xp = x;
            let mut xm = x;
            xp[i] += 1e-6;
            xm[i] -= 1e-6;
            let fd = (allocation_cost(&m, &g, &s, xp.map(Jet::c)).unwrap().v
                - allocation_cost(&m, &g, &s, xm.map(Jet::c)).unwrap().v)
                / 2e-6;
            assert!(
                (ad.d[i] - fd).abs() < 1e-5,
                "dimension {i}: {} != {fd}",
                ad.d[i]
            );
        }
    }
    #[test]
    fn optimizer_is_monotone_and_bounded() {
        let m = Acoustics::new(Profile::Catfish);
        let path = optimize_single(&m, &SingleSettings::default());
        assert!(path.len() > 2);
        for w in path.windows(2) {
            assert!(w[1].cost <= w[0].cost);
            assert!((AMIN..=AMAX).contains(&w[1].a));
            assert!((FMIN..=FMAX).contains(&w[1].f));
        }
    }
    #[test]
    fn refinement_acceptance_and_deadband() {
        let m = Acoustics::new(Profile::Catfish);
        let g = Geometry::demo();
        let s = AllocSettings::default();
        let (r, _) = refine(&m, &g, &s);
        let a = allocation(&m, &g, &r, r.z, r.frequencies);
        assert!(a.feasible);
        assert!(a.cost <= allocation(&m, &g, &s, [0.; 2], [1.5; 4]).cost);
        let bad = AllocSettings { deadband: 1.2, ..s };
        assert!(!allocation(&m, &g, &bad, [0.; 2], [1.5; 4]).feasible);
    }
    #[test]
    fn integrated_psd_matches_surrogate() {
        let m = Acoustics::new(Profile::Salmon);
        let df = 44100. / 4096.;
        let p: f64 = (1..=2048)
            .map(|i| {
                let hz = i as f64 * df;
                m.psd(hz, 0.9, 1.2, 0.3) * m.profile.weight(hz) * df
            })
            .sum();
        assert!((10. * p.log10() - m.value(0.9, 1.2, 0.3)).abs() < 1e-8);
    }
}
