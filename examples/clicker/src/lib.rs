mod init;
mod state;
use state::State;
use winit::{
    event::*, event_loop::{EventLoop, EventLoopWindowTarget}, keyboard::{Key, NamedKey}
};

use state::{Widget, WidgetType};

#[cfg(target_arch = "wasm32")]
#[allow(unused)]
use init::parse_url_query_string;

pub fn run() {
    // Init
    init::init_logger();
    let (event_loop, window) = init::init_window();

    let widgets: Vec<Widget> = (0..10000).map(|n| {
        Widget::new([(n/100) as f32/50.0-1.0, (n/100) as f32/50.0-0.98, (n%100) as f32/50.0-1.0, (n%100) as f32/50.0-0.98], WidgetType::EllipticButton)
    }).collect();
    // widgets.iter().for_each(|w| {
    //     println!("{}", w.limits[2]);
    // });

    let mut state = pollster::block_on(State::new(window, widgets));

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
            } if window_id == state.window.id() => if !state.input(event) {
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

                        let (mapped_id_buffer, width, height) = state.mapped_id_buffer(); // DEBUG
                    
                        let slice: &[u8] = &mut mapped_id_buffer.slice(..).get_mapped_range(); // DEBUG
                        let mut vec: Vec<u8> = Vec::with_capacity(slice.len() / 4); // DEBUG
                        slice.into_iter().step_by(4).for_each(|&val| vec.push(val)); // DEBUG
                        image::save_buffer("./id_buffer.png", vec.as_slice(), width, height, image::ColorType::L8).unwrap(); // DEBUG

                        target.exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);

                        state.window.request_redraw();
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

                        // state.window.request_redraw();
                    }
                    _ => {}
                }
            },
            _ => {}
        }
    });
}