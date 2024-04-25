use std::sync::Arc;

use winit::{dpi::PhysicalSize, event_loop::EventLoop, window::Window};

pub(super) fn init_logger() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            // As we don't have an environment to pull logging level from, we use the query string.
            let query_string = web_sys::window().unwrap().location().search().unwrap();
            let query_level: Option<log::LevelFilter> = parse_url_query_string(&query_string, "RUST_LOG")
                .and_then(|x| x.parse().ok());

            // We keep wgpu at Error level, as it's very noisy.
            let base_level = query_level.unwrap_or(log::LevelFilter::Info);
            let wgpu_level = query_level.unwrap_or(log::LevelFilter::Error);

            // On web, we use fern, as console_log doesn't have filtering on a per-module level.
            fern::Dispatch::new()
                .level(base_level)
                .level_for("wgpu_core", wgpu_level)
                .level_for("wgpu_hal", wgpu_level)
                .level_for("naga", wgpu_level)
                .chain(fern::Output::call(console_log::log))
                .apply()
                .unwrap();
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        } else {
            // parse_default_env will read the RUST_LOG environment variable and apply it on top
            // of these default filters.
            env_logger::builder()
                .filter_level(log::LevelFilter::Info)
                // We keep wgpu at Error level, as it's very noisy.
                .filter_module("wgpu_core", log::LevelFilter::Info)
                .filter_module("wgpu_hal", log::LevelFilter::Error)
                .filter_module("naga", log::LevelFilter::Error)
                .parse_default_env()
                .init();
        }
    }
}

pub(super) fn init_window() -> (EventLoop<()>, Arc<Window>) {
    let event_loop = EventLoop::new().unwrap();
    let mut builder = winit::window::WindowBuilder::new();
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowBuilderExtWebSys;
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        builder = builder.with_canvas(Some(canvas));
    }
    builder = builder.with_inner_size(PhysicalSize { width: 1000, height: 1000 }).with_title("A cross-platform GUI app :D");
    let window = Arc::new(builder.build(&event_loop).unwrap());

    (event_loop, window)
}