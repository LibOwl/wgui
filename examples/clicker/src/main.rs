mod init;
mod state;
use state::State;
use winit::{
    event::*, event_loop::{EventLoop, EventLoopWindowTarget}, keyboard::{Key, NamedKey}
};

fn main() {
    // Init
    init::init_logger();
    let (event_loop, window) = init::init_window();
    let winref = &window.clone();
    let mut state = pollster::block_on(State::new(winref));

    // Run loop
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            use winit::platform::web::EventLoopExtWebSys;
            let event_loop_function = EventLoop::spawn;
        } else {
            let event_loop_function = EventLoop::run;
        }
    }

    log::info!("Entering event loop...");
    // On native this is a result, but on wasm it's a unit type.
    #[allow(clippy::let_unit_value)]
    let _ = (event_loop_function)(
        event_loop,
        move |event: Event<()>, target: &EventLoopWindowTarget<()>| 
    {

        // let _ = (&state.instance, &state.adapter, &state.shader, &state.pipeline_layout);

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                match event {
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key: Key::Named(NamedKey::Escape),
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);

                        window.request_redraw();
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        
                    }
                    WindowEvent::RedrawRequested => {
                        state.update();
                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => target.exit(),
                            Err(e) => eprintln!("{:?}", e),
                        }

                        // window.request_redraw();
                    }
                    _ => {}
                }
            },
            _ => {}
        }
    });
}