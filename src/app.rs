use crate::{ad::Jet, canvas::*, model::*};
use eframe::egui::{self, Color32, RichText, Vec2};
use egui_plot::{Legend, Line, Plot, PlotImage, PlotPoint, Points};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const CHAPTERS: [&str; 4] = [
    "Auditory weighting",
    "Single-fin optimization",
    "Four-fin allocation",
    "Experimental evidence",
];
const CAPTIONS: [&str; 4] = [
    "Frequency-selective hearing changes which actuation command is preferred.",
    "A soft force penalty permits motion across constant-force contours.",
    "Body-frame target, baseline allocation and null-space redistribution.",
    "Received sound decreases; physical tracking must still be evaluated independently.",
];
include!("views.rs");
include!("export.rs");
include!("graphics.rs");

#[derive(Serialize, Deserialize)]
struct Preset {
    version: u32,
    profile: Profile,
    single: SingleSettings,
    allocation: AllocSettings,
}
struct Recording {
    dir: PathBuf,
    frame: usize,
    total: usize,
    fps: usize,
    qa: bool,
    prepared: bool,
    settle: usize,
}

pub struct Studio {
    tab: usize,
    math_mode: bool,
    selected_fin: usize,
    profile: Profile,
    model: Acoustics,
    single: SingleSettings,
    alloc: AllocSettings,
    geometry: Geometry,
    path: Vec<Step>,
    cursor: usize,
    phase: f64,
    playing: bool,
    speed: f64,
    story: bool,
    presentation: bool,
    fullscreen: bool,
    surface_mode: bool,
    field: usize,
    show_vectors: bool,
    matrix_mode: bool,
    evidence_metric: usize,
    camera: Camera,
    robot_camera: Camera,
    status: String,
    texture: Option<egui::TextureHandle>,
    null_texture: Option<egui::TextureHandle>,
    recording: Option<Recording>,
    screenshot: Option<PathBuf>,
    capture_button: bool,
    last: Instant,
    tick: f64,
    export_seconds: usize,
    export_fps: usize,
    qa_exit: bool,
    frames: usize,
}
impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>, args: &[String]) -> Self {
        let ctx = &cc.egui_ctx;
        ctx.set_theme(egui::Theme::Dark);
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = PANEL;
        style.visuals.window_fill = PANEL;
        style.visuals.extreme_bg_color = BG;
        style.visuals.faint_bg_color = Color32::from_rgb(22, 35, 51);
        style.visuals.override_text_color = Some(Color32::from_rgb(225, 234, 244));
        style.visuals.selection.bg_fill = Color32::from_rgb(34, 90, 91);
        style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, TEAL);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(27, 43, 60);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 68, 81);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(38, 95, 94);
        style.spacing.item_spacing = Vec2::new(10., 7.);
        style.spacing.button_padding = Vec2::new(12., 7.);
        style.spacing.slider_width = 154.;
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(20.));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(11.));
        ctx.set_style_of(egui::Theme::Dark, style);
        let model = Acoustics::new(Profile::Catfish);
        let single = SingleSettings::default();
        let path = optimize_single(&model, &single);
        let qa = args.iter().any(|v| v == "--capture-all");
        let tour = args.iter().any(|v| v == "--export-tour");
        let number_arg = |key: &str, default: usize, min: usize, max: usize| {
            args.windows(2)
                .find(|w| w[0] == key)
                .and_then(|w| w[1].parse::<usize>().ok())
                .unwrap_or(default)
                .clamp(min, max)
        };
        let seconds = number_arg("--seconds", 24, 4, 120);
        let fps = number_arg("--fps", 30, 10, 60);
        let out = args
            .windows(2)
            .find(|w| w[0] == "--out")
            .map(|w| PathBuf::from(&w[1]))
            .unwrap_or_else(|| PathBuf::from("captures"));
        let record = if qa || tour {
            Some(Recording {
                dir: out,
                frame: 0,
                total: if qa { 10 } else { seconds * fps },
                fps,
                qa,
                prepared: false,
                settle: 0,
            })
        } else {
            None
        };
        Self {
            tab: 0,
            math_mode: false,
            selected_fin: 0,
            profile: Profile::Catfish,
            model,
            single,
            alloc: AllocSettings::default(),
            geometry: Geometry::demo(),
            path,
            cursor: 0,
            phase: 0.,
            playing: !(qa || tour),
            speed: 1.,
            story: false,
            presentation: false,
            fullscreen: false,
            surface_mode: false,
            field: 0,
            show_vectors: false,
            matrix_mode: false,
            evidence_metric: 0,
            camera: Camera::default(),
            robot_camera: Camera::default(),
            status: "Illustrative model".into(),
            texture: None,
            null_texture: None,
            recording: record,
            screenshot: None,
            capture_button: false,
            last: Instant::now(),
            tick: 0.,
            export_seconds: 24,
            export_fps: 30,
            qa_exit: qa || tour,
            frames: 0,
        }
    }
    fn rebuild(&mut self) {
        self.model = Acoustics::new(self.profile);
        self.path = optimize_single(&self.model, &self.single);
        self.cursor = 0;
        self.tick = 0.;
    }
    fn current(&self) -> Step {
        self.path[self.cursor.min(self.path.len() - 1)]
    }
    fn section(ui: &mut egui::Ui, title: &str) {
        ui.add_space(12.);
        ui.label(RichText::new(title).size(10.).color(MUTED).strong());
        ui.add_space(2.);
    }
    fn note(ui: &mut egui::Ui, text: &str) {
        ui.label(RichText::new(text).small().color(MUTED));
    }
    fn formula(ui: &mut egui::Ui, equation: &str, reference: &str) {
        egui::Frame::new()
            .fill(BG)
            .corner_radius(7.)
            .inner_margin(12.)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(equation).monospace().size(16.).color(TEAL));
                    ui.label(RichText::new(reference).small().color(MUTED));
                });
            });
    }
    fn metric(ui: &mut egui::Ui, label: &str, value: String, detail: &str, color: Color32) {
        egui::Frame::new()
            .fill(PANEL)
            .corner_radius(8.)
            .inner_margin(12.)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new(label).small().color(MUTED));
                ui.label(RichText::new(value).size(25.).color(color));
                ui.label(RichText::new(detail).small().color(MUTED));
            });
    }
    fn top(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top")
            .exact_height(55.)
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(22, 13)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("SilentSwim").size(20.).color(TEAL));
                    });
                    ui.add_space(28.);
                    if !self.presentation {
                        for (i, short) in [
                            "01  Hearing",
                            "02  Single fin",
                            "03  Allocation",
                            "04  Evidence",
                        ]
                        .iter()
                        .enumerate()
                        {
                            if ui.selectable_label(self.tab == i, *short).clicked() {
                                self.tab = i;
                                self.story = false;
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(if self.presentation {
                                "Exit stage [P]"
                            } else {
                                "Stage [P]"
                            })
                            .clicked()
                        {
                            self.presentation = !self.presentation;
                        }
                        if ui
                            .selectable_label(self.math_mode, "Math")
                            .on_hover_text("Equations, matrices and detailed annotations")
                            .clicked()
                        {
                            self.math_mode = !self.math_mode;
                        }
                        if ui
                            .button("PNG")
                            .on_hover_text("Save the complete rendered view (Ctrl+S)")
                            .clicked()
                        {
                            self.capture_button = true;
                        }
                        if ui
                            .button(if self.playing { "Pause" } else { "Play" })
                            .clicked()
                        {
                            self.playing = !self.playing;
                        }
                    });
                });
            });
    }
    fn sidebar(&mut self, ctx: &egui::Context) {
        if self.presentation {
            return;
        }
        egui::SidePanel::left("controls").exact_width(286.).resizable(false).frame(egui::Frame::new().fill(PANEL).inner_margin(16.)).show(ctx,|ui|{
            ui.style_mut().spacing.slider_width = 94.;
            ui.style_mut().spacing.button_padding = Vec2::new(7.,4.);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            egui::ScrollArea::vertical().show(ui,|ui|{
                ui.set_width(244.);
                ui.label(RichText::new("EXPERIMENT CONTROLS").size(11.).color(TEAL).strong());
                Self::section(ui,"ACOUSTIC OBJECTIVE");
                let old=self.profile;
                egui::ComboBox::from_id_salt("profile").width(210.).selected_text(self.profile.label()).show_ui(ui,|ui|{for p in Profile::ALL{ui.selectable_value(&mut self.profile,p,p.label());}});
                let mut changed=old!=self.profile;
                Self::note(ui,"Illustrative sensitivity profiles; compare commands within one profile.");
                match self.tab {
                    0=>{
                        Self::section(ui,"PROBE COMMAND");
                        changed|=ui.add(egui::Slider::new(&mut self.single.start[0],AMIN..=AMAX).text("A [rad]")).changed();
                        changed|=ui.add(egui::Slider::new(&mut self.single.start[1],FMIN..=FMAX).text("f [Hz]")).changed();
                        changed|=ui.add(egui::Slider::new(&mut self.single.theta,-std::f64::consts::PI..=std::f64::consts::PI).text("center [rad]")).changed();
                        Self::section(ui,"REFERENCE");Self::note(ui,"Amber: A = 0.80 rad, f = 1.50 Hz, center = 0. Cyan: current command. PSD uses a fixed digital reference.");
                        Self::section(ui,"READ THE EQUATIONS");Self::note(ui,"Thresholds are interpolated in log frequency. Subtracting the minimum threshold normalizes peak sensitivity to one. Outside its support the weight is zero.");
                    },
                    1=>{
                        Self::section(ui,"FORCE & COST WEIGHTS");
                        changed|=ui.add(egui::Slider::new(&mut self.single.target,0.0..=2.8).text("F* [N]")).changed();
                        changed|=ui.add(egui::Slider::new(&mut self.single.penalty,0.1..=1000.).logarithmic(true).text("lambda F")).changed();
                        changed|=ui.add(egui::Slider::new(&mut self.single.acoustic,0.0..=3.).text("lambda H")).changed();
                        changed|=ui.add(egui::Slider::new(&mut self.single.regularization,0.0..=2.).text("lambda R")).changed();
                        changed|=ui.add(egui::Slider::new(&mut self.single.theta,-std::f64::consts::PI..=std::f64::consts::PI).text("center [rad]")).changed();
                        Self::section(ui,"LANDSCAPE");
                        egui::ComboBox::from_id_salt("field").selected_text(["Acoustic score L","Total objective J1","Force residual"][self.field]).show_ui(ui,|ui|{for (i,t) in ["Acoustic score L","Total objective J1","Force residual"].iter().enumerate(){ui.selectable_value(&mut self.field,i,*t);}});
                        ui.checkbox(&mut self.surface_mode,"3D mathematical surface");ui.checkbox(&mut self.show_vectors,"Negative gradient field");
                        Self::note(ui,"Click the 2D map to set the initial command. Amber curves show constant model force; they are not constraints.");
                        Self::section(ui,"SOLVER PATH");
                        if ui.button("Restart descent").clicked(){self.cursor=0;self.tick=0.;self.playing=true;}
                        let max=self.path.len()-1;ui.add(egui::Slider::new(&mut self.cursor,0..=max).text("step"));
                        if ui.button("Step once [Right]").clicked(){self.cursor=(self.cursor+1).min(max);self.playing=false;}
                        Self::note(ui,"Projected Adam with monotone backtracking. Gradients use forward automatic differentiation.");
                    },
                    2=>{
                        Self::note(ui,"Edit the six body-frame targets above the plots.");
                        Self::section(ui,"NULL-SPACE COORDINATES");
                        ui.add(egui::Slider::new(&mut self.alloc.z[0],-1.1..=1.1).text("z1 [N]"));ui.add(egui::Slider::new(&mut self.alloc.z[1],-1.1..=1.1).text("z2 [N]"));
                        Self::section(ui,"FREQUENCY / ANALYTICAL INVERSION");
                        for i in 0..4 {ui.add(egui::Slider::new(&mut self.alloc.frequencies[i],FMIN..=FMAX).text(format!("f{} [Hz]",i+1)));}
                        if ui.button("Refine 6 variables").clicked(){let (r,status)=refine(&self.model,&self.geometry,&self.alloc);self.alloc=r;self.status=status;}
                        if ui.button("Restore nominal").clicked(){self.alloc.restore_reference();self.status="Nominal allocation restored.".into();}
                        Self::section(ui,"POST-PROCESSING CHECK");ui.add(egui::Slider::new(&mut self.alloc.deadband,0.0..=1.2).text("A deadband"));
                        ui.checkbox(&mut self.matrix_mode,"Show B, N and wrench values");Self::note(ui,"Red regions are inadmissible at the selected frequencies. Wrench residual is recomputed from final commands, after deadband.");
                    },
                    _=>{
                        Self::section(ui,"PAPER RESULTS");
                        egui::ComboBox::from_id_salt("evidence_metric").selected_text(["Catfish-weighted score","Salmon-weighted score","Broadband level","Measured force error"][self.evidence_metric]).show_ui(ui,|ui|{for (i,t) in ["Catfish-weighted score","Salmon-weighted score","Broadband level","Measured force error"].iter().enumerate(){ui.selectable_value(&mut self.evidence_metric,i,*t);}});
                        Self::note(ui,"Values transcribed from Tables I–II. Bars show means; whiskers show ±1 SD, not confidence intervals. No raw trajectories were supplied.");
                        Self::section(ui,"VALIDATION");Self::note(ui,"Single fin: n = 87\nMAE 2.29 dB; RMSE 2.76 dB\nR² = 0.718; Spearman = 0.835\n\nFour fins: n = 20\nMAE 1.34 dB; RMSE 1.63 dB\nR² = 0.625\n\nSix-variable refinement on Jetson: 75.2 ± 12.2 ms per update.");
                    }
                }
                if changed {self.rebuild();}
                Self::section(ui,"PLAYBACK & EXPORT");
                ui.add(egui::Slider::new(&mut self.speed,0.1..=2.).text("speed"));
                if ui.checkbox(&mut self.story,"Automatic chapter tour").changed(){self.phase=0.;}
                ui.horizontal(|ui|{if ui.button("Save scene").clicked(){self.save_preset();}
if ui.button("Load scene").clicked(){self.load_preset();}});
                if ui.button("Export numerical CSV").clicked(){self.export_csv();}
                ui.collapsing("Video / frame sequence",|ui|{
                    ui.add(egui::Slider::new(&mut self.export_seconds,4..=120).text("seconds"));ui.add(egui::Slider::new(&mut self.export_fps,10..=60).text("fps"));
                    if self.recording.is_none(){if ui.button("Export deterministic tour").clicked(){self.start_recording();}}else if ui.button("Cancel export").clicked(){self.recording=None;self.status="Export cancelled; completed frames remain available.".into();}
                    Self::note(ui,"Exports PNG frames at fixed simulation times, plus an FFmpeg command for MP4. Stage layout is used automatically.");
                });
                ui.add_space(10.);Self::note(ui,"Space play/pause · 1–4 chapters\nP stage · F11 fullscreen · Ctrl+S PNG\nIllustrative k0 = 1 N/Hz²");
            });
        });
    }
    fn heading(&self, ui: &mut egui::Ui, _kicker: &str, title: &str, subtitle: &str) {
        ui.heading(title).on_hover_text(subtitle);
        ui.add_space(8.);
    }
    fn hearing(&mut self, ui: &mut egui::Ui, height: f32) {
        self.heading(ui,"01 / PERCEPTION -> OBJECTIVE","Auditory weighting","Synthetic spectrum and example threshold knots; all integrals and weights are computed live.");
        let a = self.single.start[0];
        let f = self.single.start[1];
        let theta = self.single.theta;
        let current = self.model.value(a, f, theta);
        let reference = self.model.value(0.8, 1.5, 0.);
        let broad = Acoustics::new(Profile::Broadband);
        ui.columns(3, |c| {
            Self::metric(
                &mut c[0],
                "SELECTED WEIGHTED SCORE",
                format!("{current:.2} dB"),
                "relative digital reference",
                TEAL,
            );
            Self::metric(
                &mut c[1],
                "CHANGE FROM REFERENCE",
                format!("{:+.2} dB", current - reference),
                "same profile / same acquisition scale",
                GOLD,
            );
            Self::metric(
                &mut c[2],
                "BROADBAND CHANGE",
                format!(
                    "{:+.2} dB",
                    broad.value(a, f, theta) - broad.value(0.8, 1.5, 0.)
                ),
                "a different objective can prefer another command",
                BLUE,
            );
        });
        ui.add_space(10.);
        Self::formula(
            ui,
            "w_s(ν) = 10^(-(T_s(ν) - min T_s)/10)      L_s = 10 log10(∫ P(ν;u) w_s(ν) dν + ε)",
            "Eqs. 2–3",
        );
        let ph = (height * 0.29).clamp(175., 260.);
        ui.columns(2, |c| {
            c[0].label("Relative threshold · log-frequency interpolation");
            Plot::new("threshold")
                .height(ph)
                .legend(Legend::default())
                .x_axis_label("log10 acoustic frequency [Hz]")
                .y_axis_label("Relative threshold [dB]")
                .show(&mut c[0], |p| {
                    for (profile, color) in [(Profile::Catfish, TEAL), (Profile::Salmon, GOLD)] {
                        p.line(
                            Line::new(
                                profile.label(),
                                profile
                                    .knots()
                                    .iter()
                                    .map(|(x, y)| [x.log10(), *y])
                                    .collect::<Vec<_>>(),
                            )
                            .color(color)
                            .width(2.5_f32),
                        );
                    }
                });
            c[1].label("Auditory power weight · zero outside profile support");
            Plot::new("weight")
                .height(ph)
                .x_axis_label("log10 acoustic frequency [Hz]")
                .y_axis_label("Normalized weight")
                .include_y(0.)
                .include_y(1.)
                .show(&mut c[1], |p| {
                    for (profile, color) in [
                        (Profile::Catfish, TEAL),
                        (Profile::Salmon, GOLD),
                        (Profile::Broadband, BLUE),
                    ] {
                        p.line(
                            Line::new(
                                profile.label(),
                                (0..500)
                                    .map(|i| {
                                        let x = 1. + i as f64 / 499. * 3.35;
                                        [x, profile.weight(10f64.powf(x))]
                                    })
                                    .collect::<Vec<_>>(),
                            )
                            .color(color)
                            .width(if profile == self.profile {
                                3.0_f32
                            } else {
                                1.0_f32
                            }),
                        );
                    }
                });
        });
        ui.columns(2, |c| {
            c[0].label("Power spectral density · probe vs reference");
            Plot::new("psd")
                .height(ph)
                .legend(Legend::default())
                .x_axis_label("log10 acoustic frequency [Hz]")
                .y_axis_label("PSD [dB digital²/Hz]")
                .show(&mut c[0], |p| {
                    for (aa, ff, tt, label, color, weighted) in [
                        (a, f, theta, "Probe PSD", TEAL, false),
                        (0.8, 1.5, 0., "Reference PSD", GOLD, false),
                        (a, f, theta, "Probe PSD × weight", BLUE, true),
                    ] {
                        let data = (0..500)
                            .map(|i| {
                                let x = 1. + i as f64 / 499. * 3.35;
                                let hz = 10f64.powf(x);
                                let mut power = self.model.psd(hz, aa, ff, tt);
                                if weighted {
                                    power *= self.model.profile.weight(hz);
                                }
                                [x, 10. * power.max(1e-14).log10()]
                            })
                            .collect::<Vec<_>>();
                        p.line(Line::new(label, data).color(color).width(2.0_f32));
                    }
                });
            c[1].label("Cumulative contribution to the weighted score");
            let df = 44100. / 4096.;
            let mut sum = 0.;
            let mut data = Vec::new();
            for i in 1..=2048 {
                let hz = i as f64 * df;
                sum += self.model.psd(hz, a, f, theta) * self.model.profile.weight(hz) * df;
                data.push([hz.log10(), sum]);
            }
            for p in &mut data {
                p[1] = p[1] / sum * 100.;
            }
            Plot::new("cumulative")
                .height(ph)
                .x_axis_label("log10 acoustic frequency [Hz]")
                .y_axis_label("Accumulated weighted power [%]")
                .include_y(0.)
                .include_y(100.)
                .show(&mut c[1], |p| {
                    p.line(
                        Line::new("Fraction of weighted energy", data)
                            .color(TEAL)
                            .width(3.0_f32),
                    );
                });
        });
        Self::note(
            ui,
            "Flapping frequency f (0.3–2.3 Hz) and acoustic frequency ν (up to 22.05 kHz) are different quantities. Relative pressure-weighted scores do not measure animal audibility or particle-motion sensitivity.",
        );
    }
    fn field_value(&self, a: f64, f: f64) -> f64 {
        match self.field {
            0 => self.model.value(a, f, self.single.theta),
            1 => single_cost(&self.model, &self.single, Jet::c(a), Jet::c(f)).v,
            _ => force(a, f) - self.single.target,
        }
    }
    fn single_view(&mut self, ui: &mut egui::Ui, height: f32) {
        self.heading(ui,"02 / DIFFERENTIABLE COMMAND SELECTION","Single-fin optimization","A and f are independent optimization variables. The force term is a soft penalty, not an equality constraint.");
        let p = self.current();
        ui.columns(4, |c| {
            Self::metric(
                &mut c[0],
                "COMMAND",
                format!("{:.3} / {:.3}", p.a, p.f),
                "amplitude [rad] / frequency [Hz]",
                TEAL,
            );
            Self::metric(
                &mut c[1],
                "PREDICTED ACOUSTIC SCORE",
                format!("{:.2} dB", p.score),
                "illustrative differentiable acoustic model",
                BLUE,
            );
            Self::metric(
                &mut c[2],
                "MODEL FORCE RESIDUAL",
                format!("{:+.4} N", p.force - self.single.target),
                "F(A,f) − F*",
                GOLD,
            );
            Self::metric(
                &mut c[3],
                "COST GRADIENT NORM",
                format!("{:.3}", p.grad[0].hypot(p.grad[1])),
                "forward AD; unscaled A/f coordinates",
                MUTED,
            );
        });
        ui.add_space(8.);
        Self::formula(
            ui,
            "F(A,f) = k0 f² (1 - cos A)     J1 = λh L_s + λF (F - F*)² + λR R(A,f)",
            "Eqs. 6–7",
        );
        let h = (height * 0.35).clamp(225., 335.);
        let n = 65;
        let mut grid = vec![vec![0.; n]; n];
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for (i, row) in grid.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = self.field_value(
                    AMIN + (AMAX - AMIN) * j as f64 / (n - 1) as f64,
                    FMIN + (FMAX - FMIN) * i as f64 / (n - 1) as f64,
                );
                lo = lo.min(*v);
                hi = hi.max(*v);
            }
        }
        ui.horizontal(|ui| {
            ui.label(
                [
                    "Acoustic score landscape [dB, rel.]",
                    "Total objective landscape",
                    "Signed model force residual [N]",
                ][self.field],
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.selectable_value(&mut self.surface_mode, true, "3D surface");
                ui.selectable_value(&mut self.surface_mode, false, "2D contours");
            });
        });
        if self.surface_mode {
            let path = self
                .path
                .iter()
                .take(self.cursor + 1)
                .map(|v| {
                    [
                        (v.f - FMIN) / (FMAX - FMIN) * 2. - 1.,
                        (v.a - AMIN) / (AMAX - AMIN) * 1.6 - 0.8,
                        self.field_value(v.a, v.f),
                    ]
                })
                .collect::<Vec<_>>();
            ui.columns(2, |c| {
                surface(&mut c[0], h, &mut self.camera, &grid, [lo, hi], &path);
                self.differential_view(&mut c[1], p);
            });
        } else {
            let mut pixels = Vec::with_capacity(n * n);
            for j in (0..n).rev() {
                for row in &grid {
                    pixels.push(palette((row[j] - lo) / (hi - lo).max(1e-9)));
                }
            }
            let image = egui::ColorImage::new([n, n], pixels);
            if let Some(t) = &mut self.texture {
                t.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.texture = Some(ui.ctx().load_texture(
                    "single-landscape",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
            let tex = self.texture.as_ref().unwrap().id();
            let resp = Plot::new("landscape")
                .height(h)
                .x_axis_label("Flapping frequency f [Hz]")
                .y_axis_label("Amplitude A [rad]")
                .include_x(FMIN)
                .include_x(FMAX)
                .include_y(AMIN)
                .include_y(AMAX)
                .allow_scroll(false)
                .show(ui, |plot| {
                    plot.image(PlotImage::new(
                        "Field",
                        tex,
                        PlotPoint::new((FMIN + FMAX) / 2., (AMIN + AMAX) / 2.),
                        Vec2::new((FMAX - FMIN) as f32, (AMAX - AMIN) as f32),
                    ));
                    for target in [0.15, 0.35, 0.65, 1.0, 1.5, 2.0, self.single.target] {
                        let points = (0..180)
                            .filter_map(|i| {
                                let f = FMIN + (FMAX - FMIN) * i as f64 / 179.;
                                inverse(target, f).filter(|a| *a >= AMIN).map(|a| [f, a])
                            })
                            .collect::<Vec<_>>();
                        if !points.is_empty() {
                            plot.line(
                                Line::new(format!("F = {target:.2} N"), points)
                                    .color(if target == self.single.target {
                                        GOLD
                                    } else {
                                        Color32::from_white_alpha(85)
                                    })
                                    .width(if target == self.single.target {
                                        2.5_f32
                                    } else {
                                        1.0_f32
                                    }),
                            );
                        }
                    }
                    if self.show_vectors {
                        let mut origins = Vec::new();
                        let mut tips = Vec::new();
                        for i in 1..14 {
                            for j in 1..10 {
                                let f = FMIN + (FMAX - FMIN) * i as f64 / 14.;
                                let a = AMIN + (AMAX - AMIN) * j as f64 / 10.;
                                let st = single_step(&self.model, &self.single, a, f);
                                let norm = (st.grad[1] * (FMAX - FMIN))
                                    .hypot(st.grad[0] * (AMAX - AMIN))
                                    .max(1e-12);
                                origins.push([f, a]);
                                tips.push([
                                    f - st.grad[1] * (FMAX - FMIN).powi(2) / norm * 0.025,
                                    a - st.grad[0] * (AMAX - AMIN).powi(2) / norm * 0.025,
                                ]);
                            }
                        }
                        plot.arrows(
                            egui_plot::Arrows::new("−∇J1 (normalized)", origins, tips)
                                .color(Color32::from_white_alpha(165)),
                        );
                    }
                    plot.line(
                        Line::new(
                            "Optimization path",
                            self.path
                                .iter()
                                .take(self.cursor + 1)
                                .map(|v| [v.f, v.a])
                                .collect::<Vec<_>>(),
                        )
                        .color(GOLD)
                        .width(3.0_f32),
                    );
                    plot.points(
                        Points::new("Current command", vec![[p.f, p.a]])
                            .color(Color32::WHITE)
                            .radius(5.5_f32),
                    );
                    plot.points(
                        Points::new(
                            "Initial command",
                            vec![[self.single.start[1], self.single.start[0]]],
                        )
                        .color(GOLD)
                        .radius(4.0_f32),
                    );
                    if plot.response().clicked() {
                        plot.pointer_coordinate()
                    } else {
                        None
                    }
                });
            if let Some(v) = resp.inner {
                self.single.start = [v.y.clamp(AMIN, AMAX), v.x.clamp(FMIN, FMAX)];
                self.rebuild();
            }
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("COLOR RANGE  {lo:.2} -> {hi:.2}"))
                    .monospace()
                    .small()
                    .color(MUTED),
            );
            ui.label(
                RichText::new("Amber: requested-force contour & descent path")
                    .small()
                    .color(GOLD),
            );
        });
        ui.columns(2, |c| {
            c[0].label("Objective decrease along accepted steps");
            Plot::new("convergence")
                .height(145.)
                .x_axis_label("Accepted optimizer step")
                .y_axis_label("J1 − J1(start)")
                .show(&mut c[0], |plot| {
                    plot.line(
                        Line::new(
                            "Δ objective",
                            self.path
                                .iter()
                                .enumerate()
                                .take(self.cursor + 1)
                                .map(|(i, v)| [i as f64, v.cost - self.path[0].cost])
                                .collect::<Vec<_>>(),
                        )
                        .color(TEAL)
                        .width(2.5_f32),
                    );
                });
            c[1].label("Penalty sweep · endpoints at different λF");
            let trade = (0..22)
                .map(|i| {
                    let mut s = self.single.clone();
                    s.penalty = 10f64.powf(-0.5 + i as f64 / 21. * 3.5);
                    let tr = optimize_single(&self.model, &s);
                    let q = tr.last().unwrap();
                    [(q.force - s.target).abs(), q.score]
                })
                .collect::<Vec<_>>();
            Plot::new("tradeoff")
                .height(145.)
                .x_axis_label("Absolute model-force residual [N]")
                .y_axis_label("Predicted score [dB]")
                .show(&mut c[1], |plot| {
                    plot.line(
                        Line::new("Penalty sweep (local solutions)", trade.clone())
                            .color(BLUE)
                            .width(2.0_f32),
                    );
                    plot.points(
                        Points::new("Sweep endpoint", trade)
                            .radius(2.5_f32)
                            .color(BLUE),
                    );
                    plot.points(
                        Points::new(
                            "Current",
                            vec![[(p.force - self.single.target).abs(), p.score]],
                        )
                        .radius(5.0_f32)
                        .color(GOLD),
                    );
                });
        });
        Self::note(
            ui,
            "R is the squared displacement from the initial command, normalized by command ranges. The sweep contains local optimizer results, not a certified Pareto front. Zero requested force explicitly sets A = 0.",
        );
    }
}
