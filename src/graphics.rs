impl Studio {
    fn graphic_controls(&mut self, ctx: &egui::Context) {
        if self.presentation {
            return;
        }
        egui::SidePanel::left("visual-controls").exact_width(240.).resizable(false).frame(egui::Frame::new().fill(PANEL).inner_margin(16.)).show(ctx,|ui|{
            ui.spacing_mut().slider_width=90.;ui.spacing_mut().button_padding=Vec2::new(8.,4.);
            egui::ScrollArea::vertical().show(ui,|ui|{
                let old=self.profile;let mut changed=false;
                if self.tab != 3 {
                    ui.label(RichText::new("LISTENER").color(TEAL).small());
                    egui::ComboBox::from_id_salt("visual-profile").selected_text(self.profile.label()).show_ui(ui,|ui|{for p in Profile::ALL{ui.selectable_value(&mut self.profile,p,p.label());}});
                } else { ui.label(RichText::new("PAPER CONDITIONS").color(GOLD).small()); ui.label("Catfish-weighted objective"); }
                ui.add_space(16.);
                match self.tab {
                    0=>{changed|=ui.add(egui::Slider::new(&mut self.single.start[0],AMIN..=AMAX).text("swing A")).changed();changed|=ui.add(egui::Slider::new(&mut self.single.start[1],FMIN..=FMAX).text("rate f")).changed();},
                    1=>{
                        ui.label("Force request");changed|=ui.add(egui::Slider::new(&mut self.single.target,0.0..=2.8).text("N")).changed();
                        ui.label("Force strictness");changed|=ui.add(egui::Slider::new(&mut self.single.penalty,0.1..=1000.).logarithmic(true)).on_hover_text("Larger values make force error more expensive; this remains a soft penalty.").changed();
                        ui.horizontal(|ui|{ui.selectable_value(&mut self.surface_mode,false,"2D");ui.selectable_value(&mut self.surface_mode,true,"3D");});
                        if ui.button("Replay descent").clicked(){self.cursor=0;self.playing=true;}
                        let max=self.path.len()-1;ui.add(egui::Slider::new(&mut self.cursor,0..=max).text("step"));
                        ui.checkbox(&mut self.show_vectors,"Gradient arrows");
                    },
                    2=>{
                        ui.label(RichText::new("REDISTRIBUTE").small().color(TEAL));
                        ui.add(egui::Slider::new(&mut self.alloc.z[0],-1.1..=1.1).text("z1"));ui.add(egui::Slider::new(&mut self.alloc.z[1],-1.1..=1.1).text("z2"));
                        ui.add_space(10.);ui.label("Fin frequency [Hz]");for i in 0..4{ui.add(egui::Slider::new(&mut self.alloc.frequencies[i],FMIN..=FMAX).text(format!("fin {}",i+1)));}
                        if ui.button("Find quieter allocation").clicked(){let(r,msg)=refine(&self.model,&self.geometry,&self.alloc);self.alloc=r;self.status=msg;}
                        if ui.button("Reset allocation").clicked(){self.alloc=AllocSettings::default();}
                        ui.add_space(10.);ui.label("Inspect one fin");ui.horizontal(|ui|{for i in 0..4{ui.selectable_value(&mut self.selected_fin,i,format!("{}",i+1));}});
                        ui.collapsing("Force request / deadband",|ui|{ui.add(egui::Slider::new(&mut self.alloc.surge,0.05..=0.85).text("h scale"));ui.add(egui::Slider::new(&mut self.alloc.heave,0.0..=0.65).text("v scale"));ui.add(egui::Slider::new(&mut self.alloc.deadband,0.0..=1.2).text("A cutoff"));});
                    },
                    _=>{ui.label("Reported means");Self::note(ui,"Full results and SD: Math");}
                }
                if old!=self.profile||changed{self.rebuild();}
                ui.add_space(22.);ui.checkbox(&mut self.story,"Auto tour");
                ui.collapsing("Save / export",|ui|{
                    if ui.button("Save scene").clicked(){self.save_preset();}
                    if ui.button("Load scene").clicked(){self.load_preset();}
                    if ui.button("Numerical CSV").clicked(){self.export_csv();}
                    ui.add(egui::Slider::new(&mut self.export_seconds,4..=120).text("seconds"));
                    if ui.button("Export video frames").clicked(){self.start_recording();}
                });
                ui.add_space(16.);Self::note(ui,"P: stage   Space: pause\nMath: equations & details");
            });
        });
    }
    fn graphic_view(&mut self, ui: &mut egui::Ui, height: f32) {
        match self.tab {
            0 => self.graphic_hearing(ui, height),
            1 => self.graphic_single(ui, height),
            2 => self.graphic_allocation(ui, height),
            _ => self.graphic_evidence(ui, height),
        }
    }
    fn graphic_hearing(&mut self, ui: &mut egui::Ui, height: f32) {
        ui.heading("01  Which sound matters?");
        let a = self.single.start[0];
        let f = self.single.start[1];
        ui.columns(2, |c| {
            c[0].label(RichText::new("SOUND").color(TEAL));
            Plot::new("visual-psd")
                .height(height * 0.47)
                .x_axis_label("log10 acoustic frequency [Hz]")
                .y_axis_label("PSD [dB]")
                .legend(Legend::default())
                .show(&mut c[0], |p| {
                    for (aa, ff, theta, name, color) in [
                        (a, f, self.single.theta, "Current", TEAL),
                        (0.8, 1.5, 0., "Reference", GOLD),
                    ] {
                        p.line(
                            Line::new(
                                name,
                                (0..400)
                                    .map(|i| {
                                        let x = 1. + i as f64 / 399. * 3.34;
                                        [
                                            x,
                                            10. * self
                                                .model
                                                .psd(10f64.powf(x), aa, ff, theta)
                                                .log10(),
                                        ]
                                    })
                                    .collect::<Vec<_>>(),
                            )
                            .color(color)
                            .width(2.5_f32),
                        );
                    }
                });
            c[1].label(RichText::new("LISTENER WEIGHT").color(BLUE));
            Plot::new("visual-ear")
                .height(height * 0.47)
                .x_axis_label("log10 acoustic frequency [Hz]")
                .y_axis_label("Sensitivity weight")
                .include_y(0.)
                .include_y(1.)
                .show(&mut c[1], |p| {
                    p.line(
                        Line::new(
                            "Selected listener",
                            (0..400)
                                .map(|i| {
                                    let x = 1. + i as f64 / 399. * 3.34;
                                    [x, self.profile.weight(10f64.powf(x))]
                                })
                                .collect::<Vec<_>>(),
                        )
                        .color(BLUE)
                        .width(3.0_f32),
                    );
                });
        });
        let delta = self.model.value(a, f, self.single.theta) - self.model.value(0.8, 1.5, 0.);
        ui.add_space(14.);
        visual_flow(
            ui,
            &[
                "Spectrum",
                "× hearing weight",
                "∫ weighted power",
                "Score [dB]",
            ],
        );
        ui.add_space(14.);
        ui.columns(2, |c| {
            Self::metric(
                &mut c[0],
                "WEIGHTED CHANGE",
                format!("{delta:+.2} dB"),
                "same listener",
                TEAL,
            );
            let b = Acoustics::new(Profile::Broadband);
            Self::metric(
                &mut c[1],
                "BROADBAND CHANGE",
                format!(
                    "{:+.2} dB",
                    b.value(a, f, self.single.theta) - b.value(0.8, 1.5, 0.)
                ),
                "all frequencies",
                BLUE,
            );
        });
    }
    fn graphic_single(&mut self, ui: &mut egui::Ui, height: f32) {
        ui.heading("02  One fin: quieter, with enough force");
        let p = self.current();
        let h = (height * 0.65).clamp(310., 560.);
        ui.columns(2, |c| {
            c[0].label(if self.surface_mode {
                "Score surface  ·  drag to orbit"
            } else {
                "Command map  ·  click a starting point"
            });
            let n = 65;
            let mut grid = vec![vec![0.; n]; n];
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for (i, row) in grid.iter_mut().enumerate() {
                for (j, v) in row.iter_mut().enumerate() {
                    *v = self.model.value(
                        AMIN + (AMAX - AMIN) * j as f64 / (n - 1) as f64,
                        FMIN + (FMAX - FMIN) * i as f64 / (n - 1) as f64,
                        self.single.theta,
                    );
                    lo = lo.min(*v);
                    hi = hi.max(*v);
                }
            }
            if self.surface_mode {
                let path = self
                    .path
                    .iter()
                    .take(self.cursor + 1)
                    .map(|v| {
                        [
                            (v.f - FMIN) / (FMAX - FMIN) * 2. - 1.,
                            (v.a - AMIN) / (AMAX - AMIN) * 1.6 - 0.8,
                            v.score,
                        ]
                    })
                    .collect::<Vec<_>>();
                surface(&mut c[0], h, &mut self.camera, &grid, [lo, hi], &path);
            } else {
                let mut pixels = Vec::new();
                for j in (0..n).rev() {
                    for row in &grid {
                        pixels.push(palette((row[j] - lo) / (hi - lo)));
                    }
                }
                let im = egui::ColorImage::new([n, n], pixels);
                if let Some(t) = &mut self.texture {
                    t.set(im, egui::TextureOptions::LINEAR);
                } else {
                    self.texture = Some(c[0].ctx().load_texture(
                        "graphic-single",
                        im,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                let response = Plot::new("visual-command-map")
                    .height(h)
                    .x_axis_label("f: strokes per second [Hz]")
                    .y_axis_label("A: swing angle [rad]")
                    .include_x(FMIN)
                    .include_x(FMAX)
                    .include_y(AMIN)
                    .include_y(AMAX)
                    .allow_scroll(false)
                    .show(&mut c[0], |plot| {
                        plot.image(PlotImage::new(
                            "Predicted score",
                            self.texture.as_ref().unwrap().id(),
                            PlotPoint::new(1.3, 0.8),
                            Vec2::new(2., 0.8),
                        ));
                        let contour = (0..200)
                            .filter_map(|i| {
                                let f = FMIN + 2. * i as f64 / 199.;
                                inverse(self.single.target, f)
                                    .filter(|a| *a >= AMIN)
                                    .map(|a| [f, a])
                            })
                            .collect::<Vec<_>>();
                        plot.line(
                            Line::new("Requested force", contour)
                                .color(GOLD)
                                .width(2.0_f32),
                        );
                        plot.line(
                            Line::new(
                                "Descent path",
                                self.path
                                    .iter()
                                    .take(self.cursor + 1)
                                    .map(|v| [v.f, v.a])
                                    .collect::<Vec<_>>(),
                            )
                            .color(Color32::WHITE)
                            .width(3.0_f32),
                        );
                        plot.points(
                            Points::new("Current", vec![[p.f, p.a]])
                                .color(TEAL)
                                .radius(7.0_f32),
                        );
                        if self.show_vectors {
                            let mut starts = Vec::new();
                            let mut tips = Vec::new();
                            for i in 1..10 {
                                for j in 1..9 {
                                    let f = FMIN + 2. * i as f64 / 10.;
                                    let a = AMIN + 0.8 * j as f64 / 9.;
                                    let st = single_step(&self.model, &self.single, a, f);
                                    let norm = (st.grad[1] * 2.).hypot(st.grad[0] * 0.8).max(1e-12);
                                    starts.push([f, a]);
                                    tips.push([
                                        f - st.grad[1] * 4. / norm * 0.03,
                                        a - st.grad[0] * 0.64 / norm * 0.03,
                                    ]);
                                }
                            }
                            plot.arrows(
                                egui_plot::Arrows::new("Cost descent", starts, tips)
                                    .color(Color32::from_white_alpha(160)),
                            );
                        }
                        if plot.response().clicked() {
                            plot.pointer_coordinate()
                        } else {
                            None
                        }
                    });
                if let Some(v) = response.inner {
                    self.single.start = [v.y.clamp(AMIN, AMAX), v.x.clamp(FMIN, FMAX)];
                    self.rebuild();
                }
            }
            color_scale(&mut c[0], lo, hi, "Quieter", "Louder");
            c[1].label("The selected command").on_hover_text(format!(
                "Stroke envelope drawn relative to its center angle {:.2} rad",
                self.single.theta
            ));
            fin_motion(&mut c[1], h * 0.54, p, self.single.target, self.phase);
            c[1].add_space(8.);
            force_gauge(&mut c[1], p.force, self.single.target);
            c[1].add_space(10.);
            Plot::new("graphic-descent")
                .height(h * 0.25)
                .x_axis_label("Optimizer step")
                .y_axis_label("Score [dB]")
                .show(&mut c[1], |plot| {
                    plot.line(
                        Line::new(
                            "Predicted sound",
                            self.path
                                .iter()
                                .enumerate()
                                .take(self.cursor + 1)
                                .map(|(i, v)| [i as f64, v.score])
                                .collect::<Vec<_>>(),
                        )
                        .color(TEAL)
                        .width(2.5_f32),
                    );
                });
        });
        ui.add_space(10.);
        ui.columns(3, |c| {
            Self::metric(
                &mut c[0],
                "SWING / RATE",
                format!("{:.2} rad / {:.2} Hz", p.a, p.f),
                "A / f",
                TEAL,
            );
            Self::metric(
                &mut c[1],
                "ACOUSTIC CHANGE",
                format!("{:+.2} dB", p.score - self.path[0].score),
                "relative to start",
                BLUE,
            );
            Self::metric(
                &mut c[2],
                "FORCE ERROR",
                format!("{:+.3} N", p.force - self.single.target),
                "a soft penalty permits a residual",
                GOLD,
            );
        });
    }
    fn graphic_allocation(&mut self, ui: &mut egui::Ui, height: f32) {
        ui.heading("03  Four fins: redistribute the same job");
        let a = allocation(
            &self.model,
            &self.geometry,
            &self.alloc,
            self.alloc.z,
            self.alloc.frequencies,
        );
        let ref_a = allocation(&self.model, &self.geometry, &self.alloc, [0.; 2], [1.5; 4]);
        let ratio = if self.presentation { 0.39 } else { 0.45 };
        let h = (height * ratio).clamp(220., 410.);
        ui.columns(2,|c|{
            c[0].horizontal(|ui|{ui.label(RichText::new("h: horizontal force").color(TEAL)).on_hover_text("Force along this fin's own horizontal direction; not the robot's x component.");ui.label(RichText::new("v: vertical force").color(BLUE));});
            robot_components(&mut c[0],h,&mut self.robot_camera,&self.geometry,&a,self.phase,Some(self.selected_fin));
            c[1].label("Redistribution map  ·  NOT a position map");
            let n=55;let mut pixels=vec![Color32::TRANSPARENT;n*n];let mut values=vec![None;n*n];let mut lo=f64::INFINITY;let mut hi=f64::NEG_INFINITY;
            for j in 0..n{for i in 0..n{let z=[-1.1+2.2*i as f64/(n-1) as f64,1.1-2.2*j as f64/(n-1) as f64];let candidate=allocation(&self.model,&self.geometry,&self.alloc,z,self.alloc.frequencies);if candidate.feasible{values[j*n+i]=Some(candidate.cost);lo=lo.min(candidate.cost);hi=hi.max(candidate.cost);}}}
            for (i,value) in values.iter().enumerate(){pixels[i]=value.map_or(Color32::from_rgb(86,37,54),|v|palette((v-lo)/(hi-lo).max(1e-9)));}
            let im=egui::ColorImage::new([n,n],pixels);if let Some(t)=&mut self.null_texture{t.set(im,egui::TextureOptions::NEAREST);}else{self.null_texture=Some(c[1].ctx().load_texture("graphic-map",im,egui::TextureOptions::NEAREST));}
            let r=Plot::new("graphic-null-map").height(h).x_axis_label("z1: horizontal-force exchange [N]").y_axis_label("z2: vertical-force exchange [N]").include_x(-1.1).include_x(1.1).include_y(-1.1).include_y(1.1).allow_scroll(false).show(&mut c[1],|p|{
                p.image(PlotImage::new("J4 at current frequencies",self.null_texture.as_ref().unwrap().id(),PlotPoint::new(0.,0.),Vec2::splat(2.2)));
                p.points(Points::new("Reference",vec![[0.,0.]]).radius(6.0_f32).color(GOLD));p.line(Line::new("Selected redistribution",vec![[0.,0.],self.alloc.z]).color(Color32::WHITE).width(2.0_f32));p.points(Points::new("Selected",vec![self.alloc.z]).radius(7.0_f32).color(Color32::WHITE));
                if p.response().clicked(){p.pointer_coordinate()}else{None}
            });if let Some(p)=r.inner{self.alloc.z=[p.x.clamp(-1.1,1.1),p.y.clamp(-1.1,1.1)];}
            color_scale(&mut c[1],lo,hi,"Lower J4","Higher J4");
            c[1].horizontal(|ui|{map_key(ui,GOLD,"reference");map_key(ui,Color32::WHITE,"selected");map_key(ui,Color32::from_rgb(86,37,54),"infeasible").on_hover_text("Cannot realize this force distribution with the current four frequencies, limits and deadband. The map holds all four frequencies fixed.");});
        });
        ui.add_space(8.);
        ui.columns(4, |c| {
            let max_h = (0..4).map(|i| a.q[i].abs()).fold(0.3_f64, f64::max);
            let max_v = (4..8).map(|i| a.q[i].abs()).fold(0.3_f64, f64::max);
            let scale = ((c[0].available_width() * 0.35) as f64 / max_h).min(34.0 / max_v);
            for (i, column) in c.iter_mut().enumerate() {
                if column
                    .selectable_label(self.selected_fin == i, format!("FIN {}", i + 1))
                    .clicked()
                {
                    self.selected_fin = i;
                }
                force_triangle(column, 115., a.q[i], a.q[i + 4], scale);
                column.label(format!(
                    "A {:.2} rad   f {:.2} Hz",
                    a.amplitude[i], a.frequency[i]
                ));
            }
        });
        ui.add_space(8.);
        ui.columns(2, |c| {
            wrench_bars(&mut c[0], &self.geometry, &self.alloc, &a);
            let delta = a.level.zip(ref_a.level).map(|(l, r)| l - r);
            Self::metric(
                &mut c[1],
                "ACOUSTIC CHANGE",
                delta.map_or("Inactive".into(), |v| format!("{v:+.2} dB")),
                "same listener / nominal reference",
                TEAL,
            );
            c[1].label(
                RichText::new(if a.feasible {
                    "MODEL WRENCH PRESERVED"
                } else {
                    "COMMAND REJECTED"
                })
                .color(if a.feasible { TEAL } else { RED }),
            );
        });
    }
    fn graphic_evidence(&mut self, ui: &mut egui::Ui, height: f32) {
        ui.heading("04  Measured benefit / measured tradeoff");
        ui.columns(2, |c| {
            for (i, title) in ["DEPTH", "SPEED"].iter().enumerate() {
                c[i].label(RichText::new(*title).size(23.).color(TEAL));
                let d = if i == 0 { -5.0 } else { -4.77 };
                Self::metric(
                    &mut c[i],
                    "HEARING-WEIGHTED CHANGE",
                    format!("{d:.2} dB"),
                    "paper Table II",
                    TEAL,
                );
                let (base, ours, unit): (f64, f64, &str) = if i == 0 {
                    (0.478, 0.387, "m")
                } else {
                    (0.034, 0.041, "m/s")
                };
                c[i].add_space(30.);
                c[i].label("Tracking error (RMSE)");
                let max = base.max(ours) * 1.3;
                for (name, v, col) in [
                    ("Baseline", base, GOLD),
                    ("Ours", ours, if i == 0 { TEAL } else { RED }),
                ] {
                    c[i].label(format!("{name}  {v:.3} {unit}"));
                    let (r, _) = c[i].allocate_exact_size(
                        Vec2::new(c[i].available_width(), height * 0.08),
                        egui::Sense::hover(),
                    );
                    c[i].painter().rect_filled(r, 6., PANEL);
                    c[i].painter().rect_filled(
                        egui::Rect::from_min_size(
                            r.min,
                            Vec2::new(r.width() * (v / max) as f32, r.height()),
                        ),
                        6.,
                        col.gamma_multiply(0.7),
                    );
                }
                c[i].add_space(20.);
                c[i].label(
                    RichText::new(if i == 0 {
                        "Tracking improved"
                    } else {
                        "Tracking error +20.6%"
                    })
                    .size(22.)
                    .color(if i == 0 { TEAL } else { GOLD }),
                );
            }
        });
        ui.add_space(20.);
        Self::note(
            ui,
            "Reported means, n = 8 per condition. Full SD and ablation results: Math.",
        );
    }
}

fn map_key(ui: &mut egui::Ui, color: Color32, text: &str) -> egui::Response {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(10.), egui::Sense::hover());
    ui.painter().circle_filled(r.center(), 4., color);
    ui.label(text)
}
fn color_scale(ui: &mut egui::Ui, lo: f64, hi: f64, left: &str, right: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.), egui::Sense::hover());
    let p = ui.painter();
    for i in 0..100 {
        let x = r.left() + r.width() * i as f32 / 100.;
        p.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, r.top()), Vec2::new(r.width() / 100. + 1., 7.)),
            0.,
            palette(i as f64 / 99.),
        );
    }
    let text = if lo.is_finite() {
        format!("{left}  {lo:.2}")
    } else {
        "No feasible point".into()
    };
    p.text(
        r.left_bottom(),
        egui::Align2::LEFT_BOTTOM,
        text,
        egui::FontId::proportional(11.),
        MUTED,
    );
    if hi.is_finite() {
        p.text(
            r.right_bottom(),
            egui::Align2::RIGHT_BOTTOM,
            format!("{hi:.2}  {right}"),
            egui::FontId::proportional(11.),
            MUTED,
        );
    }
}
fn force_triangle(ui: &mut egui::Ui, height: f32, h: f64, v: f64, scale: f64) {
    let (r, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let p = ui.painter_at(r);
    p.rect_filled(r, 6., PANEL);
    let o = egui::pos2(r.center().x - 20., r.center().y + 12.);
    let x = o + Vec2::new((h * scale) as f32, 0.);
    let y = o + Vec2::new(0., (-v * scale) as f32);
    let end = egui::pos2(x.x, y.y);
    arrow(&p, o, x, TEAL, 2.5);
    arrow(&p, o, y, BLUE, 2.5);
    arrow(&p, o, end, GOLD, 2.);
    p.line_segment(
        [x, end],
        egui::Stroke::new(1.0_f32, MUTED.gamma_multiply(0.35)),
    );
    p.text(
        r.left_top() + Vec2::new(8., 7.),
        egui::Align2::LEFT_TOP,
        format!("h {h:+.2} N"),
        egui::FontId::monospace(11.),
        TEAL,
    );
    p.text(
        r.left_bottom() + Vec2::new(8., -6.),
        egui::Align2::LEFT_BOTTOM,
        format!("v {v:+.2} N"),
        egui::FontId::monospace(11.),
        BLUE,
    );
}
fn force_gauge(ui: &mut egui::Ui, actual: f64, target: f64) {
    let max = actual.max(target).max(0.1) * 1.15;
    ui.label("Force: cyan = current / amber = request");
    for (v, col) in [(target, GOLD), (actual, TEAL)] {
        let (r, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.), egui::Sense::hover());
        ui.painter().rect_filled(r, 4., PANEL);
        ui.painter().rect_filled(
            egui::Rect::from_min_size(r.min, Vec2::new(r.width() * (v / max) as f32, 20.)),
            4.,
            col.gamma_multiply(0.7),
        );
        ui.painter().text(
            r.left_center() + Vec2::new(7., 0.),
            egui::Align2::LEFT_CENTER,
            format!("{v:.3} N"),
            egui::FontId::monospace(12.),
            Color32::WHITE,
        );
    }
}
fn fin_motion(ui: &mut egui::Ui, height: f32, s: Step, target: f64, t: f64) {
    let (r, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let p = ui.painter_at(r);
    p.rect_filled(r, 8., PANEL);
    let o = egui::pos2(r.center().x, r.top() + height * 0.27);
    let len = height * 0.43;
    let angle = s.a * (std::f64::consts::TAU * s.f * t).sin();
    let point = |a: f64| o + Vec2::new(a.sin() as f32 * len, a.cos() as f32 * len);
    let arc = (0..60)
        .map(|i| point(-s.a + 2. * s.a * i as f64 / 59.))
        .collect::<Vec<_>>();
    p.add(egui::Shape::line(arc, egui::Stroke::new(2.0_f32, BLUE)));
    for a in [-s.a, s.a] {
        p.line_segment(
            [o, point(a)],
            egui::Stroke::new(1.0_f32, BLUE.gamma_multiply(0.5)),
        );
    }
    let tip = point(angle);
    p.line_segment([o, tip], egui::Stroke::new(14.0_f32, TEAL));
    p.circle_filled(o, 9., Color32::WHITE);
    p.text(
        r.center_top() + Vec2::new(0., 12.),
        egui::Align2::CENTER_TOP,
        format!("A = {:.2} rad       f = {:.2} Hz", s.a, s.f),
        egui::FontId::proportional(17.),
        TEAL,
    );
    let origin = egui::pos2(r.left() + 25., r.bottom() - 24.);
    let scale = (r.width() - 100.) / s.force.max(target).max(0.1) as f32;
    arrow(
        &p,
        origin,
        origin + Vec2::new(s.force as f32 * scale, 0.),
        GOLD,
        3.,
    );
    p.text(
        origin - Vec2::new(0., 16.),
        egui::Align2::LEFT_BOTTOM,
        "Model force",
        egui::FontId::proportional(12.),
        GOLD,
    );
}
fn wrench_bars(ui: &mut egui::Ui, g: &Geometry, s: &AllocSettings, a: &Allocation) {
    ui.label("Same body job?  outline = reference / fill = final");
    let reference = g.b * s.qref();
    let mut q = Q::zeros();
    for i in 0..4 {
        let f = force(a.amplitude[i], a.frequency[i]);
        q[i] = f * a.theta[i].cos();
        q[i + 4] = f * a.theta[i].sin();
    }
    let w = g.b * q;
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 98.), egui::Sense::hover());
    let p = ui.painter_at(r);
    for i in 0..6 {
        let col = if a.feasible { TEAL } else { RED };
        let width = r.width() / 6.;
        let x = r.left() + i as f32 * width;
        let center = egui::pos2(x + width * 0.5, r.top() + 43.);
        let max = reference[i].abs().max(w[i].abs()).max(0.05);
        let to_y = |v: f64| center.y - (v / max) as f32 * 28.;
        p.line_segment(
            [
                egui::pos2(x + 5., center.y),
                egui::pos2(x + width - 5., center.y),
            ],
            egui::Stroke::new(1.0_f32, MUTED),
        );
        let rr = egui::Rect::from_two_pos(
            egui::pos2(center.x - 12., center.y),
            egui::pos2(center.x + 12., to_y(reference[i])),
        );
        p.rect_stroke(
            rr,
            0.,
            egui::Stroke::new(1.5_f32, GOLD),
            egui::StrokeKind::Outside,
        );
        let rr = egui::Rect::from_two_pos(
            egui::pos2(center.x - 8., center.y),
            egui::pos2(center.x + 8., to_y(w[i])),
        );
        p.rect_filled(rr, 0., col);
        p.text(
            egui::pos2(center.x, r.bottom() - 17.),
            egui::Align2::CENTER_BOTTOM,
            ["Fx", "Fy", "Fz", "Tx", "Ty", "Tz"][i],
            egui::FontId::proportional(12.),
            MUTED,
        );
        p.text(
            egui::pos2(center.x, r.bottom()),
            egui::Align2::CENTER_BOTTOM,
            format!("{:+.2}", w[i] - reference[i]),
            egui::FontId::monospace(11.),
            col,
        );
    }
}
fn visual_flow(ui: &mut egui::Ui, labels: &[&str]) {
    ui.columns(labels.len(), |cols| {
        for (i, l) in labels.iter().enumerate() {
            egui::Frame::new()
                .fill(PANEL)
                .inner_margin(15.)
                .corner_radius(8.)
                .show(&mut cols[i], |ui| {
                    ui.label(
                        RichText::new(*l)
                            .size(18.)
                            .color(if i == 0 { TEAL } else { BLUE }),
                    );
                });
        }
    });
}
