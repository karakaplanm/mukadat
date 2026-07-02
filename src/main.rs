pub mod app;
pub mod calibration;
pub mod image_ops;
pub mod model;
pub mod project_io;

fn main() {
    dioxus::launch(app::App);
}
