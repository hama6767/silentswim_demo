# Mathematical scope and provenance

The supplied manuscript is the conceptual source. The interactive application is an **illustrative theory implementation authorized to use synthetic data**, not a reproduction of trained SilentSwim models or a closed-loop robot simulator. Paper figures, recordings and PDF contents are not distributed.

## Auditory objective (Eqs. 2–3)

Threshold knots are interpolated linearly in `log(frequency)`. The relative minimum threshold is zero for every illustrative profile, so `w = 10^(-T/10)`, with zero weight outside the knot support.

| Profile | Illustrative frequency [Hz] : relative threshold [dB] knots |
| --- | --- |
| Catfish-like | 50:35, 100:20, 200:10, 500:0, 1000:4, 2000:14, 4000:32 |
| Salmon-like | 30:24, 50:12, 100:0, 200:5, 400:22, 700:40 |
| Broadband | 10:0, 22050:0 |

These are illustrative knots, **not digitized or validated species audiograms**.

The synthetic PSD is a sum of three normalized lognormal spectral bases centered at 110, 850 and 4200 Hz (log-standard deviations 0.35, 0.42 and 0.45), plus a flat background. Their positive coefficients are smooth functions of amplitude, flapping frequency and center angle, defined explicitly in `Acoustics::components`. They create competing low-, mid- and high-frequency preferences. No learned network is claimed. The differentiable coefficient/basis factorization computes the same integral as directly summing PSD bins, with analytic automatic derivatives.

The integral uses positive frequency bins `ν_k = k * 44100 / 4096`, `k = 1..2048`, and rectangle quadrature in **linear frequency and linear power**. The logarithmic chart axis is for presentation only. Floor `epsilon = 1e-18`. This matches the paper's frequency resolution but is not a Welch estimator or an audio file analysis feature. Current and reference comparisons use one profile and the same digital reference. Cross-profile absolute scores are not interpreted as comparable audibility measures.

## Single-fin soft penalty (Eqs. 6–7)

`F(A,f) = k0 * f² * (1 - cos(A))`, with illustrative `k0 = 1 N/Hz²`.

The supported active command domain is `A ∈ [0.4,1.2] rad`, `f ∈ [0.3,2.3] Hz`. Center angle is fixed within each solve. The cost is `lambda_h L + lambda_F (F-F*)² + lambda_R R`, where `R` is squared displacement from the initial command, normalized by the A and f ranges. This regularizer is an explicit demo choice because the paper does not fully specify its single-fin form.

Forward-mode automatic differentiation gives command gradients. Up to 100 projected Adam steps use monotone backtracking; a steepest-descent direction replaces momentum if it points uphill. The demo solver is designed to make a readable trajectory and **does not reproduce the paper's single-fin optimizer timing**. The path is not constrained to a constant-force curve. Zero requested force is an explicit inactive command (`A = 0`), outside the active amplitude domain. The penalty sweep is a collection of local solutions, not a certified Pareto frontier. The 3D view additionally shows acoustic, force and objective derivatives and a symmetrized Hessian computed by central differences of AD gradients (step 1e-4); its eigenvalues describe local curvature in unscaled A/f coordinates, not a proof of global optimality. Surface coordinates are normalized and stretched horizontally to fit the panel.

## Four-fin allocation (Eqs. 8–19)

The illustrative symmetric geometry has fin positions `(±0.65, ±0.40, 0) m`. Horizontal fin directions are `(c,c,0), (c,-c,0), (c,-c,0), (c,c,0)`, where `c = 1/sqrt(2)`. Vertical directions are `(0,0,1)`. Each column of `B` consists of force direction followed by `position × direction`. Its rank is six. Force vector ordering is `[h1,h2,h3,h4,v1,v2,v3,v4]`.

The two orthonormal null vectors have nonzero components `[1,1,-1,-1]/2` in the horizontal block and `[1,-1,-1,1]/2` in the vertical block. Thus `BN = 0`, and `q = q_ref + Nz` preserves the full modeled wrench. In v0.3 the user enters body-frame `w* = [Fx,Fy,Fz,Mx,My,Mz]`. The baseline is `q_ref = Bᵀ(BBᵀ)⁻¹ w*`, evaluated analytically for this fixed geometry. Its six rows are orthogonal. This is a minimum-norm allocator without actuator saturation optimization: a rejected baseline is not proof that every distribution is infeasible. Targets are never silently scaled. Legacy scenes without `body_target` retain the original h/v-scaled q_ref. New defaults request [1.6,0,1.1,0,0,0]. Center angles use rotation convention `r_i = +1` for this illustrative geometry.

Force magnitude is `sqrt(h_i² + v_i²)`. For active fins, the frequency interval is:

```text
lower = max(f_min, sqrt(F_i / (k0 * (1 - cos(A_max)))))
upper = min(f_max, sqrt(F_i / (k0 * (1 - cos(A_min)))))
```

The baseline frequency is 1.5 Hz clamped to each fin's admissible interval; baseline A/f regularizers and acoustic comparisons use these same frequencies. An empty interval is rejected. Amplitude is `acos(1 - F_i/(k0*f_i²))`. Zero-force fins use zero amplitude and are excluded from the acoustic sum. If all fins are inactive the score is displayed as inactive, without inventing a finite silence measurement.

Four-fin scores are composed with a numerically stable log-sum of **linear powers**. The displayed example model already produces dB; a real standardized network must first be de-standardized. Eq. 19 uses illustrative `mu_L = -40 dB` and `sigma_L = 10 dB`, `q_scale = max(mean(abs(q_ref)),0.05 N)`, and weights from the paper: `lambda_h=1`, `lambda_q=0.05`, `lambda_A=0.02`, `lambda_f,reg=0.02`. Command range normalization and the 1/8 and 1/4 factors are applied explicitly.

The demo refinement evaluates deterministic null/frequency seed combinations plus the current command as a warm-start seed, then uses **four Adam steps at learning rate 0.06** per seed. Null variables use `z = 1.1*tanh(x)`. Frequencies use a tanh map into their allocation-dependent feasible intervals. Forward AD differentiates through the moving endpoints, force inversion, center angle, acoustic composition and all regularizers. Piecewise min/max derivatives follow the active branch; derivatives at switching boundaries are not unique.

Candidates are accepted only if they improve the nominal objective and pass actuator and final-wrench checks. Otherwise the nominal command is retained. The nominal reference is explicitly checked before optimization. The interactive sliders intentionally allow probing invalid commands. Infeasible inversions yield zero displayed amplitude and a rejected state, rather than silently clipping a force-preserving solution. After the amplitude deadband, force is reconstructed from final A/f/theta before testing wrench equality. The norm mixes forces and moments and is labeled accordingly; the matrix view also gives each component with its own units.

The animation is a fin command schematic with arbitrary common phase, not a hydrodynamic or acoustic phase simulation. The application does not imply that model-wrench preservation establishes identical physical forces, trajectories or biological outcomes.

## Reported evidence

The Evidence page transcribes Table I (n=30 per method), Table II (n=8 per task/controller), and the held-out aggregate validation statistics. Whiskers are **one standard deviation**, not confidence intervals. No per-trial samples, trajectories, spectra or spectrograms are reconstructed from these aggregates. Negative lower mean-minus-SD bounds for a nonnegative force-error metric are descriptive intervals, not negative error observations.

## Numerical checks

- Fin force/inversion round trips at active-domain boundaries; zero force; empty intervals.
- Rank(B)=6, BN=0, NᵀN=I and six-dimensional wrench invariance.
- Equal four-source incoherent sum adds `10 log10(4)` dB; inactive sum handling.
- Log-frequency audiogram interpolation and zero support.
- Single-fin and six-variable AD compared to central finite differences away from kinks.
- Monotone accepted single-fin descent and domain limits.
- Four-fin improvement/feasibility acceptance and post-deadband rejection.
- Direct spectral quadrature agrees with factorized differentiable score.
- Invalid scene versions, non-finite values and out-of-range commands are rejected.

- Body-frame right inverse agrees with a numerical matrix inverse; signed six-axis, zero, and high valid requests survive force inversion/refinement.
- Impossible requests remain unchanged and are rejected; legacy scenes retain their original baseline.
