# 90-second expert demo outline

All interactive model scenes should retain the **ILLUSTRATIVE MODEL** badge. Use the Evidence page for claims about measured improvements.

| Time | Action | Narrative |
| --- | --- | --- |
| 0–20 s | Hearing; vary the probe command and switch Catfish-like/Salmon-like | The cost is a pressure-PSD integral weighted by a frequency-selective audiogram. Distinguish flapping frequency from acoustic frequency. Compare commands within a profile. |
| 20–40 s | Single fin; enable gradient arrows, restart descent, raise lambda F | Amplitude and frequency are independent. A soft force penalty trades a residual against acoustic cost; the optimizer can cross constant-force contours. Show the 3D surface briefly. |
| 40–60 s | Allocation; enter Fx/Fy/Fz and Mx/My/Mz, inspect baseline forces, adjust z1/z2, then refine | Start from the body-frame target and its minimum-norm baseline B⁺w*. B has rank six and two null directions. Changing z redistributes fin forces; frequency selects an operating point, while amplitude follows analytical force inversion. |
| 60–72 s | Show matrices; raise amplitude deadband | Bq alone is not a sufficient acceptance check. Reconstruct the wrench from final commands after deadband; reject invalid candidates and keep the nominal command. |
| 72–90 s | Evidence | Report 5.00 and 4.77 dB weighted reductions. Depth RMSE improves, but speed RMSE rises by 20.6%. Model-force invariance and physical tracking are separate claims. |

For a clean video, use P for stage mode, F11 for full screen, and an external recorder, or use the built-in deterministic PNG tour export. Exact PNG sequence timing is independent of workstation speed. Encode with the generated `encode-mp4.ps1` (Windows) or `sh encode-mp4.sh` (Linux) using FFmpeg.

