#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod ad;
mod app;
mod canvas;
mod model;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("SilentSwim Studio | Hearing-aware control allocation")
            .with_inner_size([1540., 960.])
            .with_min_inner_size([1100., 700.]),
        renderer: eframe::Renderer::Glow,
        multisampling: 4,
        ..Default::default()
    };
    eframe::run_native(
        "SilentSwim Studio",
        options,
        Box::new(move |cc| Ok(Box::new(app::Studio::new(cc, &args)))),
    )
}
