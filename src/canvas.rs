use crate::model::{Allocation, Geometry};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

pub const TEAL: Color32 = Color32::from_rgb(72, 222, 195);
pub const GOLD: Color32 = Color32::from_rgb(255, 195, 100);
pub const BLUE: Color32 = Color32::from_rgb(116, 161, 255);
pub const MUTED: Color32 = Color32::from_rgb(143, 162, 185);
pub const BG: Color32 = Color32::from_rgb(11, 18, 30);
pub const PANEL: Color32 = Color32::from_rgb(17, 27, 42);
pub const RED: Color32 = Color32::from_rgb(255, 117, 130);

pub fn palette(t: f64) -> Color32 {
    let stops = [
        [27., 33., 74.],
        [47., 78., 124.],
        [36., 140., 153.],
        [87., 196., 157.],
        [240., 220., 123.],
    ];
    let x = t.clamp(0., 1.) * 3.9999;
    let i = x as usize;
    let f = x - i as f64;
    Color32::from_rgb(
        (stops[i][0] * (1. - f) + stops[i + 1][0] * f) as u8,
        (stops[i][1] * (1. - f) + stops[i + 1][1] * f) as u8,
        (stops[i][2] * (1. - f) + stops[i + 1][2] * f) as u8,
    )
}
pub fn arrow(p: &egui::Painter, a: Pos2, b: Pos2, color: Color32, width: f32) {
    p.line_segment([a, b], Stroke::new(width, color));
    let v = (b - a).normalized();
    let n = Vec2::new(-v.y, v.x);
    let tip = 8.0f32.min(a.distance(b) * 0.3);
    p.line_segment([b, b - v * tip + n * tip * 0.45], Stroke::new(width, color));
    p.line_segment([b, b - v * tip - n * tip * 0.45], Stroke::new(width, color));
}
#[derive(Clone, Copy)]
pub struct Camera {
    pub yaw: f64,
    pub pitch: f64,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -0.6,
            pitch: 0.55,
        }
    }
}
impl Camera {
    fn project(self, v: [f64; 3], rect: Rect, scale: f64) -> Pos2 {
        let x = v[0] * self.yaw.cos() - v[1] * self.yaw.sin();
        let y = v[0] * self.yaw.sin() + v[1] * self.yaw.cos();
        let yy = y * self.pitch.sin() - v[2] * self.pitch.cos();
        Pos2::new(
            rect.center().x + (x * scale) as f32,
            rect.center().y + (yy * scale) as f32,
        )
    }
    fn depth(self, v: [f64; 3]) -> f64 {
        (v[0] * self.yaw.sin() + v[1] * self.yaw.cos()) * self.pitch.cos() + v[2] * self.pitch.sin()
    }
    fn interact(&mut self, r: &egui::Response) {
        if r.dragged() {
            let d = r.drag_delta();
            self.yaw += d.x as f64 * 0.009;
            self.pitch = (self.pitch + d.y as f64 * 0.007).clamp(0.08, 1.5);
        }
        if r.double_clicked() {
            *self = Self::default();
        }
    }
}

pub fn surface(
    ui: &mut egui::Ui,
    height: f32,
    cam: &mut Camera,
    grid: &[Vec<f64>],
    range: [f64; 2],
    path: &[[f64; 3]],
) {
    let (r, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::drag());
    cam.interact(&response);
    let p = ui.painter_at(r);
    p.rect_filled(r, 8., BG);
    // Fit the full 3D bounding box to the canvas at every camera angle.
    let mut bounds = Rect::NOTHING;
    for x in [-1.1, 1.2] {
        for y in [-0.9, 1.1] {
            for z in [-0.5, 1.1] {
                bounds.extend_with(cam.project([x, y, z], r, 1.0));
            }
        }
    }
    let scale =
        ((r.width() - 90.) / bounds.width()).min((r.height() - 65.) / bounds.height()) as f64;
    // Plot axes are normalized; use extra horizontal space without clipping.
    let sx = (scale * 1.6).min(((r.width() - 70.) / bounds.width()) as f64) as f32;
    let proj = |v| {
        let unit = cam.project(v, r, 1.0) - bounds.center();
        r.center() + Vec2::new(unit.x * sx, unit.y * scale as f32)
    };
    let nx = grid.len();
    let ny = grid[0].len();
    let dz = (range[1] - range[0]).max(1e-9);
    let vertex = |i: usize, j: usize| {
        [
            (i as f64 / (nx - 1) as f64 - 0.5) * 2.,
            (j as f64 / (ny - 1) as f64 - 0.5) * 1.6,
            ((grid[i][j] - range[0]) / dz - 0.35) * 1.25,
        ]
    };
    for k in 0..=8 {
        let x = k as f64 / 4. - 1.;
        p.line_segment(
            [proj([x, -0.8, -0.44]), proj([x, 0.8, -0.44])],
            Stroke::new(0.6_f32, Color32::from_rgb(40, 52, 71)),
        );
    }
    let mut cells = Vec::new();
    for i in 0..nx - 1 {
        for j in 0..ny - 1 {
            let v = vertex(i, j);
            cells.push((cam.depth(v), i, j));
        }
    }
    cells.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, i, j) in cells {
        let pts = vec![
            proj(vertex(i, j)),
            proj(vertex(i + 1, j)),
            proj(vertex(i + 1, j + 1)),
            proj(vertex(i, j + 1)),
        ];
        let color = palette((grid[i][j] - range[0]) / dz);
        p.add(egui::Shape::convex_polygon(
            pts,
            color,
            Stroke::new(0.25_f32, Color32::from_black_alpha(35)),
        ));
    }
    for w in path.windows(2) {
        let v = |a: [f64; 3]| [a[0], a[1], ((a[2] - range[0]) / dz - 0.35) * 1.25 + 0.025];
        p.line_segment([proj(v(w[0])), proj(v(w[1]))], Stroke::new(3.0_f32, GOLD));
    }
    let axes = [
        ([-1., -0.8, -0.44], [1.15, -0.8, -0.44], "Frequency f [Hz]"),
        ([-1., -0.8, -0.44], [-1., 1., -0.44], "Amplitude A [rad]"),
        ([-1., -0.8, -0.44], [-1., -0.8, 1.1], "Objective"),
    ];
    for (a, b, label) in axes {
        arrow(&p, proj(a), proj(b), MUTED, 1.3);
        p.text(
            proj(b),
            egui::Align2::CENTER_TOP,
            label,
            egui::FontId::proportional(12.),
            MUTED,
        );
    }
    p.text(
        r.left_top() + Vec2::new(14., 12.),
        egui::Align2::LEFT_TOP,
        "DRAG TO ORBIT  /  DOUBLE-CLICK TO RESET",
        egui::FontId::monospace(10.),
        MUTED,
    );
}

pub fn robot(
    ui: &mut egui::Ui,
    height: f32,
    cam: &mut Camera,
    g: &Geometry,
    a: &Allocation,
    t: f64,
) {
    robot_components(ui, height, cam, g, a, t, None);
}

#[allow(clippy::too_many_arguments)]
pub fn robot_components(
    ui: &mut egui::Ui,
    height: f32,
    cam: &mut Camera,
    g: &Geometry,
    a: &Allocation,
    t: f64,
    selected: Option<usize>,
) {
    let (r, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::drag());
    cam.interact(&response);
    let p = ui.painter_at(r);
    p.rect_filled(r, 8., BG);
    let scale = r.width().min(r.height() * 1.6) as f64 * 0.33;
    let proj = |v| cam.project(v, r, scale);
    for k in -5..=5 {
        let x = k as f64 * 0.3;
        p.line_segment(
            [proj([x, -1.2, -0.5]), proj([x, 1.2, -0.5])],
            Stroke::new(0.6_f32, Color32::from_rgb(30, 48, 66)),
        );
        p.line_segment(
            [proj([-1.5, x, -0.5]), proj([1.5, x, -0.5])],
            Stroke::new(0.6_f32, Color32::from_rgb(30, 48, 66)),
        );
    }
    let hull = vec![
        proj([0.95, 0., 0.1]),
        proj([0.65, 0.29, 0.1]),
        proj([-0.7, 0.29, 0.1]),
        proj([-0.87, 0., 0.1]),
        proj([-0.7, -0.29, 0.1]),
        proj([0.65, -0.29, 0.1]),
    ];
    p.add(egui::Shape::convex_polygon(
        hull,
        Color32::from_rgb(31, 63, 82),
        Stroke::new(2.0_f32, Color32::from_rgb(84, 122, 145)),
    ));
    p.line_segment(
        [proj([-0.65, 0., 0.13]), proj([0.7, 0., 0.13])],
        Stroke::new(2.0_f32, TEAL.gamma_multiply(0.65)),
    );
    let colors = [TEAL, BLUE, GOLD, Color32::from_rgb(206, 150, 247)];
    for (i, color) in colors.into_iter().enumerate() {
        let pos = g.positions[i];
        let dir = g.directions[i];
        let flap = a.theta[i] + a.amplitude[i] * (std::f64::consts::TAU * a.frequency[i] * t).sin();
        let sign = if pos[1] > 0. { 1. } else { -1. };
        let tip = [
            pos[0] + 0.12 * flap.sin(),
            pos[1] + sign * 0.45 * flap.cos(),
            pos[2] + 0.45 * flap.sin(),
        ];
        let fin = vec![
            proj([pos[0] - 0.18, pos[1], 0.]),
            proj([pos[0] + 0.18, pos[1], 0.]),
            proj([tip[0] + 0.12, tip[1], tip[2]]),
            proj([tip[0] - 0.22, tip[1], tip[2]]),
        ];
        p.add(egui::Shape::convex_polygon(
            fin,
            color.gamma_multiply(0.45),
            Stroke::new(1.4_f32, color),
        ));
        let v = [
            pos[0] + dir[0] * a.q[i] * 0.6,
            pos[1] + dir[1] * a.q[i] * 0.6,
            pos[2] + a.q[i + 4] * 0.6,
        ];
        if selected.is_none() {
            arrow(&p, proj(pos), proj(v), color, 2.8);
        }
        if selected == Some(i) {
            let hv = [
                pos[0] + dir[0] * a.q[i] * 0.9,
                pos[1] + dir[1] * a.q[i] * 0.9,
                pos[2],
            ];
            let vv = [pos[0], pos[1], pos[2] + a.q[i + 4] * 0.9];
            let full = [hv[0], hv[1], vv[2]];
            arrow(&p, proj(pos), proj(hv), TEAL, 3.5);
            arrow(&p, proj(pos), proj(vv), BLUE, 3.5);
            arrow(&p, proj(pos), proj(full), GOLD, 2.0);
            p.circle_stroke(proj(pos), 7., Stroke::new(2.0_f32, Color32::WHITE));
            p.text(
                proj(hv) + Vec2::new(10., 12.),
                egui::Align2::LEFT_CENTER,
                "h",
                egui::FontId::proportional(22.),
                TEAL,
            );
            p.text(
                proj(vv) + Vec2::new(0., -12.),
                egui::Align2::CENTER_BOTTOM,
                "v",
                egui::FontId::proportional(22.),
                BLUE,
            );
        }
        let label = proj([pos[0], pos[1] + sign * 0.62, 0.]);
        p.text(
            label,
            egui::Align2::CENTER_CENTER,
            format!("FIN {}  {:.2} Hz", i + 1, a.frequency[i]),
            egui::FontId::monospace(11.),
            color,
        );
    }
    let origin = [-1.15, -0.7, -0.45];
    for (v, label, c) in [
        ([0.4, 0., 0.], "x", RED),
        ([0., 0.4, 0.], "y", TEAL),
        ([0., 0., 0.4], "z", BLUE),
    ] {
        let tip = std::array::from_fn(|i| origin[i] + v[i]);
        arrow(&p, proj(origin), proj(tip), c, 1.5);
        p.text(
            proj(tip),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::monospace(12.),
            c,
        );
    }
    p.text(
        r.left_top() + Vec2::new(14., 12.),
        egui::Align2::LEFT_TOP,
        "ILLUSTRATIVE GEOMETRY  /  DRAG TO ORBIT",
        egui::FontId::monospace(10.),
        MUTED,
    );
}
