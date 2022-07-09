use winit::window::WindowBuilder;

fn main() {
    tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(pathland::_main(wb_platform_specific));
}

#[cfg(unix)]
fn wb_platform_specific(wb: WindowBuilder) -> WindowBuilder {
    use winit::platform::unix::WindowBuilderExtUnix;
    wb
        .with_class("pathland".to_string(), "pathland".to_string())
        .with_app_id("pathland".to_string())

}

#[cfg(not(unix))]
fn wb_platform_specific(wb: WindowBuilder) -> WindowBuilder {
    wb
}
