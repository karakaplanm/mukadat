use crate::calibration::{Calibration, Point2D};
use crate::image_ops::load_image_as_data_url;
use crate::model::{DataPoint, ProjectData, RightTab, SortOrder};
use dioxus::prelude::{ReadableExt, Signal, WritableExt};

/// The subset of `App`'s reactive state that a saved project can restore.
/// `Signal<T>` handles are cheap to copy, so callers can build this struct
/// inline from their local signals without disturbing component state.
#[derive(Clone, Copy)]
pub struct ProjectSignals {
    pub image_path: Signal<Option<String>>,
    pub image_size: Signal<Option<(u32, u32)>>,
    pub image_data_url: Signal<Option<String>>,
    pub raw_image: Signal<Option<image::DynamicImage>>,
    pub calibration: Signal<Calibration>,
    pub points: Signal<Vec<DataPoint>>,
    pub zoom_factor: Signal<f32>,
    pub pan_offset: Signal<Point2D>,
    pub sort_order: Signal<SortOrder>,
    pub include_errors: Signal<bool>,
    pub connect_lines: Signal<bool>,
    pub marker_size: Signal<f32>,
    pub auto_trace_step: Signal<f32>,
    pub auto_trace_tol: Signal<f32>,
    pub right_tab: Signal<RightTab>,
    pub x_label: Signal<String>,
    pub y_label: Signal<String>,
    pub x1_input: Signal<String>,
    pub x2_input: Signal<String>,
    pub y1_input: Signal<String>,
    pub y2_input: Signal<String>,
}

/// Applies a loaded `ProjectData` onto the app's signals, resolving the
/// referenced image if any. Returns `Some(error)` if the image failed to
/// load, without aborting the rest of the restore.
pub fn apply_project_data(sig: &mut ProjectSignals, data: ProjectData) -> Option<String> {
    *sig.calibration.write() = data.calibration;
    *sig.points.write() = data.points;
    *sig.x_label.write() = data.x_label;
    *sig.y_label.write() = data.y_label;

    let mut image_error = None;
    match data.image_path.as_deref() {
        Some(img_path) => match load_image_as_data_url(img_path) {
            Ok((data_url, size, img)) => {
                *sig.image_path.write() = Some(img_path.to_string());
                *sig.image_size.write() = Some(size);
                *sig.image_data_url.write() = Some(data_url);
                *sig.raw_image.write() = Some(img);
            }
            Err(e) => {
                image_error = Some(format!("Error loading image: {}", e));
            }
        },
        None => {
            *sig.image_path.write() = None;
            *sig.image_size.write() = None;
            *sig.image_data_url.write() = None;
            *sig.raw_image.write() = None;
        }
    }

    if let Some(zoom) = data.zoom_factor {
        *sig.zoom_factor.write() = zoom;
    }
    if let Some(pan) = data.pan_offset {
        *sig.pan_offset.write() = Point2D {
            x: pan[0] as f64,
            y: pan[1] as f64,
        };
    }
    if let Some(sort) = data.sort_order {
        *sig.sort_order.write() = sort;
    }
    if let Some(inc_err) = data.include_errors {
        *sig.include_errors.write() = inc_err;
    }
    if let Some(conn) = data.connect_lines {
        *sig.connect_lines.write() = conn;
    }
    if let Some(m_size) = data.marker_size {
        *sig.marker_size.write() = m_size;
    }
    if let Some(at_step) = data.auto_trace_step {
        *sig.auto_trace_step.write() = at_step;
    }
    if let Some(at_tol) = data.auto_trace_tol {
        *sig.auto_trace_tol.write() = at_tol;
    }
    if let Some(t) = data.right_tab {
        *sig.right_tab.write() = t;
    }

    let cal = sig.calibration.read();
    *sig.x1_input.write() = data
        .x1_input
        .unwrap_or_else(|| cal.x1.value.map_or(String::new(), |v| v.to_string()));
    *sig.x2_input.write() = data
        .x2_input
        .unwrap_or_else(|| cal.x2.value.map_or(String::new(), |v| v.to_string()));
    *sig.y1_input.write() = data
        .y1_input
        .unwrap_or_else(|| cal.y1.value.map_or(String::new(), |v| v.to_string()));
    *sig.y2_input.write() = data
        .y2_input
        .unwrap_or_else(|| cal.y2.value.map_or(String::new(), |v| v.to_string()));

    image_error
}

/// Path to the file that remembers the most recently opened/saved project.
pub fn last_project_pointer_path() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .map(|home| format!("{}/.r3data_last_project", home))
}

pub fn read_last_project_path() -> Option<String> {
    let pointer = last_project_pointer_path()?;
    std::fs::read_to_string(pointer)
        .ok()
        .map(|s| s.trim().to_string())
}

pub fn write_last_project_path(path: &str) {
    if let Some(pointer) = last_project_pointer_path() {
        let _ = std::fs::write(pointer, path);
    }
}
