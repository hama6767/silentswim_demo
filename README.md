# SilentSwim Studio

A native **Rust + egui** mathematical demo studio for *SilentSwim: Embedding Auditory Sensitivity into Differentiable Control Allocation of Underwater Robots*. Designed for an expert-facing conference video or interactive demonstration.

**This is a theory demonstrator, not an experimental reproduction.** Interactive acoustics, audiograms, force calibration and robot geometry are explicitly illustrative. The Evidence page separately displays the aggregate measurements reported in the supplied manuscript. No unpublished PDF, learned weights or experimental recordings are bundled.

## Download and run

Download **silentswim-studio.exe** from [Releases](https://github.com/hama6767/silentswim_demo/releases/latest), then double-click it. The Windows x64 release uses a statically linked MSVC runtime. No Rust, Python, model download, or network connection is needed to run the app. A graphics driver with OpenGL support is required. Release binaries are unsigned.

The ZIP contains the same executable plus this guide and the mathematical notes. SHA256 checksums accompany each release.

## Four linked workspaces

| Workspace | Expert-facing controls and displays |
| --- | --- |
| **01 Hearing** | Log-frequency threshold interpolation; normalized auditory weights with finite support; PSD and weighted PSD; cumulative energy integral; within-profile command comparison |
| **02 Single fin** | Acoustic, total-cost and signed-force-residual heatmaps; 3D orbitable surfaces; analytical constant-force contours; negative gradient field; click-to-initialize descent; stepwise convergence; force-penalty sweep |
| **03 Allocation** | Animated four-fin geometry; two-dimensional null-space cost/feasibility map; six-variable automatic differentiation and Adam refinement; analytical amplitude inversion; per-fin frequency feasibility; B/N matrices; all six wrench components reconstructed after deadband |
| **04 Evidence** | Table I objective ablation with mean ± SD; Table II closed-loop acoustic and tracking results; explicit speed-tracking tradeoff; validation statistics |

All plots support hover inspection and normal egui_plot navigation. 3D scenes support drag-to-orbit and double-click-to-reset. The white marker is the selected command, cyan indicates the current model, and amber marks a reference or force contour. Red/burgundy means actuator or final-wrench feasibility failed.

## 発表・動画向けの使い方

1. **Hearing** で Catfish-like / Salmon-like / Broadband を切り替え、周波数重みによって評価が変わることを示します。
2. **Single fin** で等力線と勾配場を表示し、`lambda F` を動かします。2D上をクリックすると初期値が変わり、`3D surface` で数理的な地形に切り替えられます。
3. **Allocation** で `z1`, `z2` と4つの周波数を変え、`Refine 6 variables` を押します。`Show B, N and wrench values` で6成分の保存を確認できます。デッドバンドを上げると、配分レベルの保存だけでは不十分な理由を説明できます。
4. **Evidence** で実測集計結果を示します。特に速度追従RMSEの増加を含め、音響改善と追従性能を分けて説明します。

UIは国際会議でそのまま収録しやすい英語表記です。仮データの画面には `ILLUSTRATIVE MODEL` を常時表示します。

### Stage and capture

- `P`: large presentation layout, with the control sidebar hidden and a chapter caption displayed.
- `F11`: fullscreen. `Escape`: leave fullscreen/stage, or cancel an export.
- `Space`: play/pause. `1`–`4`: select chapter.
- `Right` / `Left`: step forward/backward through the single-fin solution.
- `Ctrl+S` or **PNG**: save the complete view at the current rendered resolution.
- **Automatic chapter tour**: a repeating 48-second mathematical walkthrough.
- **Video / frame sequence → Export deterministic tour**: choose duration (4–120 seconds) and frame rate (10–60 fps). The app creates a new subdirectory with a fixed-time PNG sequence and `encode-mp4.ps1`. Export duration is independent of rendering speed. Keep the window size and DPI unchanged during export.

To encode MP4, install FFmpeg with H.264 support separately, then run in PowerShell:

```powershell
./encode-mp4.ps1
# Or specify its executable explicitly:
./encode-mp4.ps1 -Ffmpeg 'C:/tools/ffmpeg/bin/ffmpeg.exe'
```

PNG capture and frame-sequence export need no external program. FFmpeg is only needed to encode the final MP4. The encoder refuses to overwrite an existing MP4.

**Save scene / Load scene** preserves the profile and mathematical control parameters as validated JSON. **Export numerical CSV** writes the optimizer path, synthetic spectrum, current allocation, scene JSON and provenance. The app does not send commands to a physical robot.

## Mathematics and provenance

See [docs/MATHEMATICS.md](docs/MATHEMATICS.md) for equations, exact illustrative parameter choices, solver differences, edge cases and test coverage. See [docs/DEMO_SCRIPT.md](docs/DEMO_SCRIPT.md) for a 90-second presentation outline.

## Build and verify

Rust stable and a native compiler/linker are required. On Windows use Rust's `x86_64-pc-windows-msvc` target and Visual Studio C++ Build Tools.

```text
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
rustfmt --edition 2024 --check src/views.rs src/export.rs
cargo run --release --locked
```

For automated **real native render capture** (six views, then exit):

```text
silentswim-studio.exe --capture-all --out captures
```

This requires a desktop graphics session; it is not a headless HTML mock. It exercises both 2D/3D single-fin views, the robot and matrix views, hearing, and reported evidence.

## GitHub Actions release

- `Verify` runs formatting, numerical tests, Clippy and a Windows build on pushes to `main` and pull requests.
- Pushing a semantic-version tag such as `v0.1.0` runs `Windows Release`: repeat verification, compile x64 MSVC with static CRT, package EXE/ZIP/checksums, then publish a GitHub Release with the workflow token.
- `Windows Release` can also be dispatched manually with an **existing tag**. Publishing is isolated in a job with `contents: write`; build jobs have read-only permissions.
- `Cargo.lock` is committed and every CI build uses `--locked`.

## Source layout

`ad.rs`: six-component forward AD. `model.rs`: acoustics, force inversion, single-fin solver and null-space allocation. `canvas.rs`: native 3D mathematical surfaces and robot schematic. `app.rs` / `views.rs`: interactive workspaces. `export.rs`: scene validation, numerical export, deterministic tour and native framebuffer capture.

