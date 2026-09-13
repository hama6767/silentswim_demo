impl Studio {
    fn differential_view(&self, ui: &mut egui::Ui, p: Step) {
        ui.label(
            RichText::new("Local differential structure")
                .size(19.)
                .color(TEAL),
        );
        Self::note(
            ui,
            "Evaluated at the current command (A, f); coordinate units are rad and Hz.",
        );
        let acoustic = self.model.score(
            Jet::var(p.a, 0),
            Jet::var(p.f, 1),
            Jet::c(self.single.theta),
        );
        let thrust = force_jet(Jet::var(p.a, 0), Jet::var(p.f, 1));
        let eps = 1e-4;
        let ap = single_step(&self.model, &self.single, p.a + eps, p.f);
        let am = single_step(&self.model, &self.single, p.a - eps, p.f);
        let fp = single_step(&self.model, &self.single, p.a, p.f + eps);
        let fm = single_step(&self.model, &self.single, p.a, p.f - eps);
        let haa = (ap.grad[0] - am.grad[0]) / (2. * eps);
        let hff = (fp.grad[1] - fm.grad[1]) / (2. * eps);
        let haf = ((ap.grad[1] - am.grad[1]) + (fp.grad[0] - fm.grad[0])) / (4. * eps);
        let spread = ((haa - hff).powi(2) + 4. * haf * haf).sqrt();
        egui::Grid::new("derivatives")
            .spacing([22., 10.])
            .show(ui, |ui| {
                ui.label("Derivative");
                ui.label("with respect to A");
                ui.label("with respect to f");
                ui.end_row();
                for (label, values) in [
                    ("Acoustic score", [acoustic.d[0], acoustic.d[1]]),
                    ("Model force", [thrust.d[0], thrust.d[1]]),
                    ("Total objective", p.grad),
                ] {
                    ui.label(label);
                    for v in values {
                        ui.monospace(format!("{v:+.5}"));
                    }
                    ui.end_row();
                }
            });
        ui.add_space(9.);
        ui.label(RichText::new("Hessian of J1 (central differences of AD gradients)").color(BLUE));
        ui.monospace(format!(
            "[ {haa:10.3}  {haf:10.3} ]\n[ {haf:10.3}  {hff:10.3} ]"
        ));
        ui.label(format!(
            "Eigenvalues: {:.3}, {:.3}",
            (haa + hff - spread) / 2.,
            (haa + hff + spread) / 2.
        ));
        Self::note(
            ui,
            "The local quadratic approximation describes curvature, not global optimality. A boundary solution need not have zero unconstrained gradient.",
        );
    }
    fn allocation_view(&mut self, ui: &mut egui::Ui, height: f32) {
        self.heading(ui,"03 / REDUNDANCY -> QUIETER ALLOCATION","Change the fin commands. Keep the modeled wrench.","Illustrative rank-six geometry, two null coordinates, four frequency variables, and exact force inversion.");
        let a = allocation(
            &self.model,
            &self.geometry,
            &self.alloc,
            self.alloc.z,
            self.alloc.frequencies,
        );
        let reference = allocation(&self.model, &self.geometry, &self.alloc, [0.; 2], [1.5; 4]);
        ui.columns(4, |c| {
            Self::metric(
                &mut c[0],
                "NULL-SPACE INVARIANT",
                format!("{:.1e}", (self.geometry.b * self.geometry.n).norm()),
                "||B N||F   /   rank(B) = 6",
                TEAL,
            );
            Self::metric(
                &mut c[1],
                "FINAL WRENCH RESIDUAL",
                format!("{:.2e}", a.residual),
                "Euclidean norm [mixed N / N·m]",
                if a.feasible { TEAL } else { RED },
            );
            Self::metric(
                &mut c[2],
                "COMPOSED ACOUSTIC CHANGE",
                if let (Some(l), Some(r)) = (a.level, reference.level) {
                    format!("{:+.2} dB", l - r)
                } else {
                    "Inactive".into()
                },
                "linear-power sum / nominal reference",
                GOLD,
            );
            Self::metric(
                &mut c[3],
                "COMMAND ACCEPTANCE",
                if a.feasible {
                    "FEASIBLE".into()
                } else {
                    "REJECT".into()
                },
                "actuator limits + final wrench check",
                if a.feasible { TEAL } else { RED },
            );
        });
        ui.add_space(8.);
        Self::formula(
            ui,
            "q = q_ref + N z     B N = 0     A_i = acos(1 - F_i / (k0 f_i²))",
            "Eqs. 11–16",
        );
        if self.matrix_mode {
            self.matrix_view(ui, &a);
        } else {
            ui.columns(2,|c|{
                c[0].label("Four-fin command realization · arbitrary animation phase");robot(&mut c[0],(height*0.32).clamp(220.,280.),&mut self.robot_camera,&self.geometry,&a,self.phase);
                c[1].label("Null-space objective J4 · frequencies held at current values");
                let n=45;let mut grid=vec![vec![None;n];n];let mut low=f64::INFINITY;let mut high=f64::NEG_INFINITY;
                for (i,row) in grid.iter_mut().enumerate(){for (j,value) in row.iter_mut().enumerate(){let z=[-1.1+2.2*i as f64/(n-1) as f64,-1.1+2.2*j as f64/(n-1) as f64];let aa=allocation(&self.model,&self.geometry,&self.alloc,z,self.alloc.frequencies);if aa.feasible{*value=Some(aa.cost);low=low.min(aa.cost);high=high.max(aa.cost);}}}
                let mut pixels=Vec::new();for j in (0..n).rev(){for row in &grid{pixels.push(row[j].map_or(Color32::from_rgb(74,34,49),|v|palette((v-low)/(high-low).max(1e-9))));}}
                let image=egui::ColorImage::new([n,n],pixels);if let Some(t)=&mut self.null_texture {t.set(image,egui::TextureOptions::NEAREST);}else{self.null_texture=Some(c[1].ctx().load_texture("null-landscape",image,egui::TextureOptions::NEAREST));}
                let tex=self.null_texture.as_ref().unwrap().id();
                let response=Plot::new("nullspace").height((height*0.32).clamp(220.,280.)).data_aspect(1.).x_axis_label("z1 [N]").y_axis_label("z2 [N]").include_x(-1.1).include_x(1.1).include_y(-1.1).include_y(1.1).allow_scroll(false).show(&mut c[1],|p|{
                    p.image(PlotImage::new("J4 / feasibility mask",tex,PlotPoint::new(0.,0.),Vec2::splat(2.2)));
                    p.points(Points::new("Nominal",vec![[0.,0.]]).radius(5.0_f32).color(GOLD));p.points(Points::new("Current",vec![self.alloc.z]).radius(6.0_f32).color(Color32::WHITE));p.line(Line::new("Redistribution",vec![[0.,0.],self.alloc.z]).color(TEAL).width(2.0_f32));
                    if p.response().clicked(){p.pointer_coordinate()}else{None}
                });
                if let Some(v)=response.inner{self.alloc.z=[v.x.clamp(-1.1,1.1),v.y.clamp(-1.1,1.1)];}
                Self::note(&mut c[1],&if low.is_finite(){format!("COLOR J4 {low:.3} -> {high:.3}  ·  Burgundy: infeasible\nClick to probe an allocation; the six-variable solver also changes f.")}else{"No feasible points at the current frequencies / deadband.".into()});
            });
        }
        Self::formula(
            ui,
            "L4 = 10 log10 Σ_i 10^(L_i/10)     J4 = (L4 - μL)/σL + allocation & command regularizers",
            "Eqs. 18–19",
        );
        ui.add_space(5.);
        egui::Grid::new("commands")
            .num_columns(7)
            .spacing([24., 7.])
            .striped(true)
            .show(ui, |ui| {
                for text in [
                    "FIN",
                    "FORCE [N]",
                    "CENTER [rad]",
                    "A [rad]",
                    "f [Hz]",
                    "ADMISSIBLE f [Hz]",
                    "SCORE [dB]",
                ] {
                    ui.label(RichText::new(text).size(10.).color(MUTED));
                }
                ui.end_row();
                for i in 0..4 {
                    ui.label(
                        RichText::new(format!("{:02}", i + 1))
                            .color([TEAL, BLUE, GOLD, Color32::LIGHT_BLUE][i]),
                    );
                    ui.monospace(format!("{:.3}", a.q[i].hypot(a.q[i + 4])));
                    ui.monospace(format!("{:.3}", a.theta[i]));
                    ui.monospace(format!("{:.3}", a.amplitude[i]));
                    ui.monospace(format!("{:.3}", a.frequency[i]));
                    ui.monospace(
                        interval(a.q[i].hypot(a.q[i + 4]))
                            .map_or("EMPTY".into(), |(lo, hi)| format!("{lo:.3} – {hi:.3}")),
                    );
                    ui.monospace(if a.active[i] {
                        format!("{:.2}", a.scores[i])
                    } else {
                        "Inactive".into()
                    });
                    ui.end_row();
                }
            });
        ui.add_space(6.);
        Self::note(
            ui,
            "Incoherent power addition omits interference, duplicated background and cross-fin coupling. Wrench preservation holds in the analytical allocation model; it does not guarantee identical physical force or tracking.",
        );
    }
    fn matrix_view(&self, ui: &mut egui::Ui, a: &Allocation) {
        ui.columns(2,|cols|{
            cols[0].label("Allocation matrix B in R^(6×8)");
            egui::Grid::new("B").spacing([8.,7.]).show(&mut cols[0],|ui|{
                for r in 0..6 {ui.label(RichText::new(["Fx","Fy","Fz","τx","τy","τz"][r]).color(TEAL));for c in 0..8{let v=self.geometry.b[(r,c)];ui.label(RichText::new(format!("{v:5.2}")).monospace().color(if v.abs()<1e-9{MUTED.gamma_multiply(0.45)}else{BLUE}));}ui.end_row();}
            });
            Self::note(&mut cols[0],"Columns: h1 h2 h3 h4 v1 v2 v3 v4\nRows: forces [N], then moments [N·m].\nFin locations: x = ±0.65 m, y = ±0.40 m.\nHorizontal directions: ±45° in each fin plane.");
            cols[1].label("Orthonormal null basis N in R^(8×2)");
            egui::Grid::new("N").spacing([22.,5.]).show(&mut cols[1],|ui|{for r in 0..8 {ui.monospace(format!("{}{}",if r<4{"h"}else{"v"},r%4+1));for c in 0..2{ui.label(RichText::new(format!("{:+.2}",self.geometry.n[(r,c)])).monospace().color(TEAL));}ui.end_row();}});
        });
        ui.add_space(12.);
        let reference = self.geometry.b * self.alloc.qref();
        let allocated = self.geometry.b * a.q;
        let mut realized = Q::zeros();
        for i in 0..4 {
            let f = force(a.amplitude[i], a.frequency[i]);
            realized[i] = f * a.theta[i].cos();
            realized[i + 4] = f * a.theta[i].sin();
        }
        let final_w = self.geometry.b * realized;
        egui::Grid::new("wrench")
            .striped(true)
            .spacing([28., 3.])
            .show(ui, |ui| {
                for t in [
                    "WRENCH",
                    "REFERENCE",
                    "B(q_ref + Nz)",
                    "FROM FINAL A, f, θ",
                    "ERROR",
                ] {
                    ui.label(RichText::new(t).small().color(MUTED));
                }
                ui.end_row();
                for i in 0..6 {
                    ui.label(
                        [
                            "Fx [N]",
                            "Fy [N]",
                            "Fz [N]",
                            "τx [N·m]",
                            "τy [N·m]",
                            "τz [N·m]",
                        ][i],
                    );
                    ui.monospace(format!("{:.6}", reference[i]));
                    ui.monospace(format!("{:.6}", allocated[i]));
                    ui.monospace(format!("{:.6}", final_w[i]));
                    ui.label(
                        RichText::new(format!("{:+.2e}", final_w[i] - reference[i]))
                            .monospace()
                            .color(if a.feasible { TEAL } else { RED }),
                    );
                    ui.end_row();
                }
            });
        ui.add_space(8.);
    }
    fn evidence(&mut self, ui: &mut egui::Ui, height: f32) {
        self.heading(ui,"04 / REPORTED EXPERIMENTAL EVIDENCE","Acoustic benefit and tracking are separate outcomes","Paper Tables I–II; values are reported means and standard deviations, not simulated trials.");
        ui.columns(3, |c| {
            Self::metric(
                &mut c[0],
                "DEPTH / CATFISH SCORE CHANGE",
                "−5.00 dB".into(),
                "baseline −50.14 -> ours −55.14 dB",
                TEAL,
            );
            Self::metric(
                &mut c[1],
                "SPEED / CATFISH SCORE CHANGE",
                "−4.77 dB".into(),
                "baseline −46.73 -> ours −51.50 dB",
                TEAL,
            );
            Self::metric(
                &mut c[2],
                "SPEED TRACKING RMSE CHANGE",
                "+20.6%".into(),
                "0.034 -> 0.041 m/s; a tracking tradeoff",
                GOLD,
            );
        });
        ui.add_space(10.);
        ui.label(RichText::new("Single-fin objective ablation").size(18.));
        let means = [
            [-27.908, -29.401, -28.578, -28.928, -30.472],
            [-42.198, -42.981, -43.239, -44.182, -43.239],
            [-17.208, -19.808, -21.101, -19.708, -20.008],
            [15.057, 19.904, 13.821, 15.123, 13.145],
        ];
        let sd = [
            [2.426, 1.931, 2.728, 2.734, 1.906],
            [2.733, 2.671, 2.981, 2.239, 2.733],
            [1.227, 2.813, 2.102, 2.926, 2.796],
            [22.345, 21.429, 19.021, 19.128, 18.357],
        ];
        let labels = [
            "Inverse / fixed f",
            "Random / catfish",
            "Gradient / broadband",
            "Gradient / salmon",
            "Gradient / catfish",
        ];
        let selected = self.evidence_metric;
        let floor = if selected == 3 {
            0.
        } else {
            means[selected]
                .iter()
                .zip(sd[selected])
                .map(|(m, s)| m - s)
                .fold(f64::INFINITY, f64::min)
                - 1.
        };
        Plot::new(("ablation", selected))
            .height((height * 0.21).clamp(145., 200.))
            .x_axis_label("Method index (see table below)")
            .y_axis_label(
                [
                    "Catfish score [dB, rel.]",
                    "Salmon score [dB, rel.]",
                    "Broadband [dBFS]",
                    "Absolute force error [%]",
                ][selected],
            )
            .include_y(floor)
            .show(ui, |p| {
                for i in 0..5 {
                    let color = if i == 4 {
                        TEAL
                    } else if i == 0 {
                        GOLD
                    } else {
                        BLUE
                    };
                    p.bar_chart(
                        egui_plot::BarChart::new(
                            labels[i],
                            vec![
                                egui_plot::Bar::new(i as f64, means[selected][i] - floor)
                                    .base_offset(floor)
                                    .width(0.5),
                            ],
                        )
                        .color(color),
                    );
                    let lo = means[selected][i] - sd[selected][i];
                    let hi = means[selected][i] + sd[selected][i];
                    p.line(
                        Line::new(
                            format!("{} ± SD", labels[i]),
                            vec![[i as f64, lo], [i as f64, hi]],
                        )
                        .color(Color32::WHITE)
                        .width(1.5_f32),
                    );
                    for y in [lo, hi] {
                        p.line(
                            Line::new("SD cap", vec![[i as f64 - 0.09, y], [i as f64 + 0.09, y]])
                                .color(Color32::WHITE),
                        );
                    }
                }
            });
        egui::Grid::new("ablation_table")
            .striped(true)
            .spacing([20., 6.])
            .show(ui, |ui| {
                for t in [
                    "INDEX / METHOD",
                    "SELECTED METRIC ± SD",
                    "FORCE ERROR [%] ± SD",
                    "OPTIMIZATION [ms] ± SD",
                ] {
                    ui.label(RichText::new(t).small().color(MUTED));
                }
                ui.end_row();
                let time = [
                    (0.707, 0.365),
                    (2.000, 0.826),
                    (40.881, 4.135),
                    (41.121, 5.624),
                    (40.926, 5.156),
                ];
                for i in 0..5 {
                    ui.label(format!("{i}   {}", labels[i]));
                    ui.monospace(format!(
                        "{:.3} ± {:.3}",
                        means[selected][i], sd[selected][i]
                    ));
                    ui.monospace(format!("{:.3} ± {:.3}", means[3][i], sd[3][i]));
                    ui.monospace(format!("{:.3} ± {:.3}", time[i].0, time[i].1));
                    ui.end_row();
                }
            });
        Self::note(
            ui,
            "n = 30 per method. Error bars are SD, not CI; a mean − SD below zero for nonnegative force error reflects the descriptive interval only. Acoustic objectives have different normalizations and are compared within columns.",
        );
        ui.add_space(12.);
        ui.label(RichText::new("Full robot / closed-loop performance").size(18.));
        egui::Grid::new("full_robot")
            .striped(true)
            .spacing([27., 8.])
            .show(ui, |ui| {
                for t in [
                    "TASK / METHOD",
                    "BROADBAND [dBFS]",
                    "CATFISH SCORE [dB, rel.]",
                    "TRACKING RMSE",
                ] {
                    ui.label(RichText::new(t).small().color(MUTED));
                }
                ui.end_row();
                for row in [
                    [
                        "Depth / baseline",
                        "−37.25 ± 0.87",
                        "−50.14 ± 0.69",
                        "0.478 ± 0.053 m",
                    ],
                    [
                        "Depth / ours",
                        "−39.68 ± 1.81",
                        "−55.14 ± 1.84",
                        "0.387 ± 0.010 m",
                    ],
                    [
                        "Speed / baseline",
                        "−35.82 ± 0.96",
                        "−46.73 ± 1.94",
                        "0.034 ± 0.007 m/s",
                    ],
                    [
                        "Speed / ours",
                        "−37.32 ± 1.65",
                        "−51.50 ± 1.63",
                        "0.041 ± 0.009 m/s",
                    ],
                ] {
                    for (j, t) in row.iter().enumerate() {
                        ui.label(RichText::new(*t).color(if j == 0 {
                            TEAL
                        } else {
                            Color32::from_rgb(220, 230, 240)
                        }));
                    }
                    ui.end_row();
                }
            });
        ui.add_space(8.);
        Self::formula(
            ui,
            "Model-wrench equality ≠ identical physical force ≠ identical closed-loop tracking",
            "Interpretation",
        );
        Self::note(
            ui,
            "n = 8 runs per task–controller condition. Fixed receiver, tank geometry and acquisition settings. These results do not establish receiver invariance, biological impact, or a worst-case runtime guarantee.",
        );
    }
}
