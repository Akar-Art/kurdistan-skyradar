#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Kurdistan SkyRadar — Airspace Command & Control (C2)")
            .with_inner_size([1440.0, 880.0])
            .with_min_inner_size([960.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Kurdistan SkyRadar C2",
        native_options,
        Box::new(|cc| Ok(Box::new(radar_app::RadarApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();

    wasm_bindgen_futures::spawn_local(async {
        use wasm_bindgen::JsCast;
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not an HtmlCanvasElement");

        let web_options = eframe::WebOptions::default();
        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(radar_app::RadarApp::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
