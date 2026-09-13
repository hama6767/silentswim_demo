impl Studio {
    fn preset(&self) -> Preset {
        Preset {
            version: 1,
            profile: self.profile,
            single: self.single.clone(),
            allocation: self.alloc.clone(),
        }
    }
    fn save_preset(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Scene JSON", &["json"])
            .set_file_name("silentswim-scene.json")
            .save_file()
        {
            self.status = match serde_json::to_string_pretty(&self.preset())
                .map_err(|e| e.to_string())
                .and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string()))
            {
                Ok(()) => format!("Scene saved: {}", path.display()),
                Err(e) => format!("Cannot save scene: {e}"),
            };
        }
    }
    fn load_preset(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Scene JSON", &["json"])
            .pick_file()
        {
            let result = std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|s| serde_json::from_str::<Preset>(&s).map_err(|e| e.to_string()))
                .and_then(|p| {
                    validate_preset(&p)?;
                    Ok(p)
                });
            match result {
                Ok(p) => {
                    self.profile = p.profile;
                    self.single = p.single;
                    self.alloc = p.allocation;
                    self.rebuild();
                    self.status = "Scene loaded.".into();
                }
                Err(e) => self.status = format!("Cannot load scene: {e}"),
            }
        }
    }
    fn export_csv(&mut self) {
        if let Some(dir) = rfd::FileDialog::new()
            .set_title("Export numerical data to folder")
            .pick_folder()
        {
            self.status = match self.write_data(&dir) {
                Ok(()) => format!("Numerical data saved: {}", dir.display()),
                Err(e) => format!("CSV export failed: {e}"),
            };
        }
    }
    fn write_data(&self, dir: &std::path::Path) -> Result<(), String> {
        let write =
            |name: &str, s: String| std::fs::write(dir.join(name), s).map_err(|e| e.to_string());
        let mut csv="step,amplitude_rad,frequency_hz,objective,predicted_score_db,model_force_n,residual_n,gradient_a,gradient_f\n".to_string();
        for (i, s) in self.path.iter().enumerate() {
            csv += &format!(
                "{i},{},{},{},{},{},{},{},{}\n",
                s.a,
                s.f,
                s.cost,
                s.score,
                s.force,
                s.force - self.single.target,
                s.grad[0],
                s.grad[1]
            );
        }
        write("single-fin-path.csv", csv)?;
        let mut csv =
            "acoustic_frequency_hz,synthetic_psd_digital_squared_per_hz,weight,weighted_psd\n"
                .to_string();
        let df = 44100. / 4096.;
        for i in 1..=2048 {
            let hz = i as f64 * df;
            let psd = self.model.psd(
                hz,
                self.single.start[0],
                self.single.start[1],
                self.single.theta,
            );
            let w = self.profile.weight(hz);
            csv += &format!("{hz},{psd},{w},{}\n", psd * w);
        }
        write("synthetic-spectrum.csv", csv)?;
        let a = allocation(
            &self.model,
            &self.geometry,
            &self.alloc,
            self.alloc.z,
            self.alloc.frequencies,
        );
        let mut csv =
            "fin,h_n,v_n,amplitude_rad,frequency_hz,center_rad,predicted_score_db,active\n"
                .to_string();
        for i in 0..4 {
            csv += &format!(
                "{},{},{},{},{},{},{},{}\n",
                i + 1,
                a.q[i],
                a.q[i + 4],
                a.amplitude[i],
                a.frequency[i],
                a.theta[i],
                if a.active[i] {
                    a.scores[i].to_string()
                } else {
                    String::new()
                },
                a.active[i]
            );
        }
        write("four-fin-allocation.csv", csv)?;
        write(
            "scene.json",
            serde_json::to_string_pretty(&self.preset()).map_err(|e| e.to_string())?,
        )?;
        write(
            "PROVENANCE.txt",
            format!(
                "All exported model values are illustrative, not experimental measurements.\nProfile: {}\nForce coefficient k0 = 1 N/Hz^2 (illustrative).\nEq. 19 normalization mu_L = -40 dB, sigma_L = 10 dB (illustrative).\nFinal modeled wrench residual norm = {:.12e} (mixed N / N*m).\nFinal command feasible = {}.\nPaper aggregate measurements are shown only in the Evidence view.\n",
                self.profile.label(),
                a.residual,
                a.feasible
            ),
        )
    }
    fn start_recording(&mut self) {
        if let Some(parent) = rfd::FileDialog::new()
            .set_title("Select parent folder for frame sequence")
            .pick_folder()
        {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let dir = parent.join(format!("silentswim-tour-{stamp}"));
            match std::fs::create_dir(&dir) {
                Ok(()) => {
                    self.recording = Some(Recording {
                        dir,
                        frame: 0,
                        total: self.export_seconds * self.export_fps,
                        fps: self.export_fps,
                        qa: false,
                        prepared: false,
                        settle: 0,
                    });
                    self.presentation = true;
                    self.playing = false;
                    self.status="Exporting deterministic tour. Escape cancels; fixed simulation time per frame.".into();
                }
                Err(e) => self.status = format!("Cannot create export folder: {e}"),
            }
        }
    }
    fn prepare_capture(&mut self) {
        let Some(r) = &self.recording else {
            return;
        };
        if r.prepared {
            return;
        }
        let (qa, frame, total, fps) = (r.qa, r.frame, r.total, r.fps);
        if qa {
            self.tab = match frame {
                0 => 0,
                1 | 2 => 1,
                3 | 4 => 2,
                _ => 3,
            };
            self.surface_mode = frame == 2;
            self.matrix_mode = frame == 4;
            self.math_mode = frame == 4;
            self.show_vectors = frame == 1;
            self.cursor = self.path.len() - 1;
            self.phase = 0.6;
            self.playing = false;
            if frame == 3 {
                let (a, status) = refine(&self.model, &self.geometry, &self.alloc);
                self.alloc = a;
                self.status = status;
            }
        } else {
            self.presentation = true;
            let t = frame as f64 / fps as f64;
            let duration = total as f64 / fps as f64;
            self.apply_tour(t, duration);
        }
        if let Some(r) = &mut self.recording {
            r.prepared = true;
            r.settle = if qa { 5 } else { 2 };
        }
    }
    fn apply_tour(&mut self, t: f64, duration: f64) {
        let normalized = (t / duration).clamp(0., 0.999999) * 4.;
        self.tab = normalized as usize;
        let u = normalized.fract();
        self.phase = t;
        match self.tab {
            0 => {
                self.single.start = [
                    0.8 + 0.2 * (u * std::f64::consts::TAU).sin(),
                    1.5 + 0.45 * (u * std::f64::consts::TAU).sin(),
                ];
            }
            1 => {
                if self.single.start != SingleSettings::default().start {
                    self.single.start = SingleSettings::default().start;
                    self.path = optimize_single(&self.model, &self.single);
                }
                self.cursor = ((u * 1.3).min(1.) * (self.path.len() - 1) as f64) as usize;
                self.surface_mode = u > 0.65;
                self.camera.yaw = -0.6 + (u - 0.65).max(0.) * 1.4;
            }
            2 => {
                let base = AllocSettings::default();
                let (opt, _) = refine(&self.model, &self.geometry, &base);
                let blend = (u * 1.5).min(1.);
                self.alloc = base.clone();
                for i in 0..2 {
                    self.alloc.z[i] = opt.z[i] * blend;
                }
                for i in 0..4 {
                    self.alloc.frequencies[i] =
                        base.frequencies[i] + (opt.frequencies[i] - base.frequencies[i]) * blend;
                }
                self.matrix_mode = u > 0.72;
            }
            _ => {}
        }
    }
    fn receive_capture(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            if let egui::Event::Screenshot { image, .. } = e {
                let Some(path) = self.screenshot.take() else {
                    continue;
                };
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                match image::save_buffer(
                    &path,
                    &bytes,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                ) {
                    Ok(()) => {
                        self.status = format!("Saved {}", path.display());
                        if let Some(r) = &mut self.recording {
                            r.frame += 1;
                            r.prepared = false;
                        }
                    }
                    Err(e) => {
                        self.status = format!("PNG export failed: {e}");
                        self.recording = None;
                        self.qa_exit = false;
                    }
                }
            }
        }
        if self.recording.as_ref().is_some_and(|r| r.frame >= r.total) {
            let r = self.recording.take().unwrap();
            if !r.qa {
                let script = format!(
                    "param([string]$Ffmpeg = 'ffmpeg')\n$ErrorActionPreference = 'Stop'\nPush-Location -LiteralPath $PSScriptRoot\ntry {{\n  & $Ffmpeg -n -framerate {} -i 'frame-%06d.png' -vf 'pad=ceil(iw/2)*2:ceil(ih/2)*2' -c:v libx264 -crf 18 -pix_fmt yuv420p 'silentswim-demo.mp4'\n  if ($LASTEXITCODE -ne 0) {{ throw 'FFmpeg encoding failed.' }}\n}} finally {{ Pop-Location }}\n",
                    r.fps
                );
                let result = std::fs::write(r.dir.join("encode-mp4.ps1"), script)
                    .map_err(|e| e.to_string())
                    .and_then(|_| self.write_data(&r.dir));
                self.status = match result {
                    Ok(()) => format!(
                        "Export complete: {} frames + encode-mp4.ps1 in {}",
                        r.total,
                        r.dir.display()
                    ),
                    Err(e) => format!("Frames saved; metadata export failed: {e}"),
                };
            }
            if self.qa_exit {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
    fn request_capture(&mut self, ctx: &egui::Context) {
        if self.capture_button && self.recording.is_none() && self.screenshot.is_none() {
            self.capture_button = false;
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG image", &["png"])
                .set_file_name(format!("silentswim-{}.png", self.tab + 1))
                .save_file()
            {
                self.screenshot = Some(path);
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
        }
        if self.screenshot.is_some() {
            return;
        }
        if let Some(r) = &mut self.recording {
            if r.settle > 0 {
                r.settle -= 1;
                return;
            }
            if let Err(e) = std::fs::create_dir_all(&r.dir) {
                self.status = format!("Cannot create capture directory: {e}");
                self.recording = None;
                self.qa_exit = false;
                return;
            }
            let name = if r.qa {
                format!("view-{:02}.png", r.frame + 1)
            } else {
                format!("frame-{:06}.png", r.frame)
            };
            self.screenshot = Some(r.dir.join(name));
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}

fn validate_preset(p: &Preset) -> Result<(), String> {
    let check = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
    let s = &p.single;
    let a = &p.allocation;
    let valid = p.version == 1
        && check(s.start[0], AMIN, AMAX)
        && check(s.start[1], FMIN, FMAX)
        && check(s.target, 0., 2.8)
        && check(s.penalty, 0.1, 1000.)
        && check(s.acoustic, 0., 3.)
        && check(s.regularization, 0., 2.)
        && check(s.theta, -std::f64::consts::PI, std::f64::consts::PI)
        && a.z.iter().all(|v| check(*v, -1.1, 1.1))
        && a.frequencies.iter().all(|v| check(*v, FMIN, FMAX))
        && check(a.heave, 0., 0.65)
        && check(a.surge, 0.05, 0.85)
        && check(a.deadband, 0., 1.2)
        && [a.q_weight, a.a_weight, a.f_weight]
            .iter()
            .all(|v| check(*v, 0., 10.));
    if valid {
        Ok(())
    } else {
        Err("Unsupported scene version or parameter outside the supported range.".into())
    }
}

impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames += 1;
        self.receive_capture(ctx);
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f64().min(0.1);
        self.last = now;
        let recording = self.recording.is_some();
        if !recording {
            // Clone before invoking other Context methods; holding an input read
            // lock while asking for focus or issuing commands can deadlock.
            let i = ctx.input(|i| i.clone());
            {
                if !ctx.wants_keyboard_input() {
                    if i.key_pressed(egui::Key::Space) {
                        self.playing = !self.playing;
                    }
                    if i.key_pressed(egui::Key::P) {
                        self.presentation = !self.presentation;
                    }
                    if i.key_pressed(egui::Key::ArrowRight) {
                        self.cursor = (self.cursor + 1).min(self.path.len() - 1);
                        self.playing = false;
                    }
                    if i.key_pressed(egui::Key::ArrowLeft) {
                        self.cursor = self.cursor.saturating_sub(1);
                        self.playing = false;
                    }
                    for (j, k) in [
                        egui::Key::Num1,
                        egui::Key::Num2,
                        egui::Key::Num3,
                        egui::Key::Num4,
                    ]
                    .iter()
                    .enumerate()
                    {
                        if i.key_pressed(*k) {
                            self.tab = j;
                            self.story = false;
                        }
                    }
                }
                if i.key_pressed(egui::Key::F11) {
                    self.fullscreen = !self.fullscreen;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
                }
                if i.modifiers.command && i.key_pressed(egui::Key::S) {
                    self.capture_button = true;
                }
                if i.key_pressed(egui::Key::Escape) {
                    self.presentation = false;
                    self.fullscreen = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                }
            }
            if self.playing {
                self.phase += dt * self.speed;
                self.tick += dt * self.speed;
                while self.tick > 0.09 {
                    self.tick -= 0.09;
                    self.cursor = (self.cursor + 1).min(self.path.len() - 1);
                }
                if self.story {
                    self.apply_tour(self.phase % 48., 48.);
                }
            }
        } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.recording = None;
            self.status = "Frame export cancelled.".into();
            self.qa_exit = false;
        }
        self.prepare_capture();
        self.top(ctx);
        egui::TopBottomPanel::bottom("status")
            .exact_height(if self.presentation { 56. } else { 34. })
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(20, 8)),
            )
            .show(ctx, |ui| {
                if self.presentation {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("{:02} / 04", self.tab + 1))
                                .color(TEAL)
                                .monospace(),
                        );
                        ui.label(RichText::new(CAPTIONS[self.tab]).size(19.));
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(if self.tab == 3 {
                                "PAPER AGGREGATES"
                            } else {
                                "ILLUSTRATIVE MODEL"
                            })
                            .color(if self.tab == 3 { GOLD } else { TEAL })
                            .small()
                            .strong(),
                        );
                        if let Some(r) = &self.recording {
                            ui.label(format!("Capture {} / {}", r.frame + 1, r.total));
                        } else {
                            ui.add(
                                egui::Label::new(RichText::new(&self.status).small().color(MUTED))
                                    .truncate(),
                            );
                        }
                    });
                }
            });
        if self.math_mode {
            self.sidebar(ctx);
        } else {
            self.graphic_controls(ctx);
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(24, 17)),
            )
            .show(ctx, |ui| {
                let height = ui.available_height();
                egui::ScrollArea::vertical()
                    .id_salt(("main-scroll", self.tab, self.matrix_mode))
                    .show(ui, |ui| {
                        if self.presentation {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(CHAPTERS[self.tab]).small().color(MUTED));
                                ui.label(
                                    RichText::new(if self.tab == 3 {
                                        "REPORTED EXPERIMENTS"
                                    } else {
                                        "ILLUSTRATIVE MODEL"
                                    })
                                    .small()
                                    .color(TEAL),
                                );
                            });
                        }
                        if !self.math_mode {
                            self.graphic_view(ui, height);
                        } else {
                            match self.tab {
                                0 => self.hearing(ui, height),
                                1 => self.single_view(ui, height),
                                2 => self.allocation_view(ui, height),
                                _ => self.evidence(ui, height),
                            }
                        }
                    });
            });
        if self.frames > 3 {
            self.request_capture(ctx);
        }
        if self.playing || self.recording.is_some() || self.screenshot.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

#[cfg(test)]
mod scene_tests {
    use super::*;
    #[test]
    fn reject_invalid_scenes() {
        let mut p = Preset {
            version: 1,
            profile: Profile::Catfish,
            single: SingleSettings::default(),
            allocation: AllocSettings::default(),
        };
        assert!(validate_preset(&p).is_ok());
        p.single.start[0] = 5.;
        assert!(validate_preset(&p).is_err());
        p.single.start[0] = 0.8;
        p.allocation.deadband = f64::NAN;
        assert!(validate_preset(&p).is_err());
    }
}
