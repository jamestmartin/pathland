mod audio;
mod graphics;

use std::sync::{Arc, RwLock};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    setup_logger();

    use winit::event_loop::EventLoop;
    use winit::platform::unix::WindowBuilderExtUnix;
    let event_loop = EventLoop::new();

    let now = std::time::Instant::now();

    let window = winit::window::WindowBuilder::new()
        // Arbitrarily chosen as the minimum resolution the game is designed to support (for e.g. UI scaling).
        .with_min_inner_size(winit::dpi::LogicalSize { height: 360, width: 640 })
        .with_title("Pathland")
        .with_maximized(true)
        // TODO: hide window until first frame is drawn (default behavior on wayland)
        .with_visible(true)
        .with_decorations(true)
        .with_class("pathland".to_string(), "pathland".to_string())
        .with_app_id("pathland".to_string())
        .build(&event_loop)
        .expect("Failed to create window.");
    // TODO: window icon, fullscreen, IME position, cursor grab, cursor visibility
    let mut graphics = graphics::Graphics::setup(window).await;
    //let audio = audio::Audio::setup();
    log::info!("Took {} milliseconds", now.elapsed().as_millis());

    event_loop.run(move |event, target, control_flow| {
        use winit::event::*;
        *control_flow = winit::event_loop::ControlFlow::Wait;
        match event {
            Event::WindowEvent { window_id, event } => {
                match event {
                    WindowEvent::CloseRequested => {
                        std::process::exit(0);
                    },
                    WindowEvent::Destroyed => {
                        std::process::exit(0);
                    },
                    WindowEvent::Focused(focused) => {
                        // TODO: handle focus/unfocus (e.g. pause, resume)
                    },
                    WindowEvent::Resized(new_size) => {
                        graphics.window_resized(new_size)
                    },
                    WindowEvent::ScaleFactorChanged { new_inner_size: new_size, .. } => {
                        graphics.window_resized(*new_size)
                    },
                    // TODO: handle user input
                    _ => {}
                }
            },
            Event::DeviceEvent { device_id, event } => {
                // TODO: handle user input
            },
            Event::MainEventsCleared => {
                // TODO: main event loop. queue simulation calculations, screen redrawing, etc.
            },
            Event::RedrawRequested(_) => {
                graphics.draw();
            },
            Event::LoopDestroyed => {
                std::process::exit(0);
            },
            _ => {}
        }
        // TODO: What is suspending/resuming? Do I want to support it?
    });
}

fn setup_logger() {
    use fern::Dispatch;
    use fern::colors::ColoredLevelConfig;
    use log::LevelFilter;

    Dispatch::new()
        .chain(
            Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{}] {}",
                        ColoredLevelConfig::default().color(record.level()),
                        message
                    ));
                })
                .level(LevelFilter::Warn)
                .level_for("pathland", LevelFilter::Info)
                .chain(std::io::stderr()))
        .chain(
            fern::Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{}] {}",
                        record.level(),
                        message
                    ))
                })
                .level(LevelFilter::Debug)
                .level_for("pathland", LevelFilter::Trace)
                .chain(std::fs::OpenOptions::new().write(true).create(true).truncate(true).open("/tmp/pathland.log").unwrap()))
        .apply().unwrap();
}
