Graphics-first update for the native Rust + egui SilentSwim mathematical demo studio.

- Default view emphasizes linked diagrams and plots; **Math** preserves the full specialist interface.
- Single-fin command map linked to animated swing amplitude, stroke rate and current/reference force bars.
- Four-fin view explicitly distinguishes local horizontal force **h** and vertical force **v**, both in N; selectable fin overlays and common-scale force triangles show their resultant.
- Redistribution map labels the z1/z2 parameter space, current selection, reference and feasibility mask; a numeric color scale identifies the fixed-frequency J4 slice.
- Six body-wrench comparisons show reference outlines, reconstructed final commands and residuals.
- Simpler visual acoustic/tracking evidence comparison, with full reported statistics still available in Math.

Existing capabilities remain available:

- Interactive auditory weighting, PSD integration and within-profile command comparisons.
- Single-fin acoustic/objective/residual heatmaps, 3D surfaces, gradients, constant-force contours and optimizer playback.
- Full six-dimensional null-space allocation, differentiable force inversion, six-variable refinement and post-deadband wrench verification.
- Reported paper evidence with mean ± SD and the acoustic/tracking tradeoff.
- Presentation mode, full screen, PNG capture, deterministic frame-sequence export, MP4 encoding script, scene JSON and numerical CSV.

Interactive data are explicitly illustrative. This is a theory demonstrator, not an experimental reproduction or robot controller.

Download **silentswim-studio.exe** to run directly, or the ZIP for the executable plus documentation. No Rust or Python installation is needed. FFmpeg is optional and only used to encode exported frames into MP4. Binaries are unsigned. SHA256SUMS.txt covers the EXE and ZIP.
