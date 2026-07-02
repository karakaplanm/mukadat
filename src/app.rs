use crate::calibration::{Calibration, Point2D};
use crate::image_ops::{
    generate_demo_graph, generate_kuva_plot_svg, get_formatted_points, load_image_as_data_url,
    trace_all_curves,
};
use crate::model::{
    DataPoint, DragTarget, PickTarget, PointSource, ProjectData, RightTab, SortOrder,
};
use crate::project_io::{
    ProjectSignals, apply_project_data, read_last_project_path, write_last_project_path,
};
use dioxus::html::PointerInteraction;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::document::eval;
use dioxus::prelude::*;
use std::time::{Duration, Instant};

/// Which side panel a splitter drag is currently resizing.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ResizePanel {
    Left,
    Right,
}

const LEFT_PANEL_MIN_WIDTH: f64 = 260.0;
const LEFT_PANEL_MAX_WIDTH: f64 = 640.0;
const RIGHT_PANEL_MIN_WIDTH: f64 = 320.0;
const RIGHT_PANEL_MAX_WIDTH: f64 = 900.0;

#[component]
pub fn App() -> Element {
    let mut image_path = use_signal(|| Option::<String>::None);
    let mut image_size = use_signal(|| Option::<(u32, u32)>::None);
    let mut image_data_url = use_signal(|| Option::<String>::None);
    let mut raw_image = use_signal(|| Option::<image::DynamicImage>::None);

    let mut calibration = use_signal(|| Calibration::default());
    let mut points = use_signal(|| Vec::<DataPoint>::new());

    let mut zoom_factor = use_signal(|| 0.8f32);
    let mut pan_offset = use_signal(|| Point2D { x: 50.0, y: 50.0 });
    let mut hovered_pixel = use_signal(|| Option::<Point2D>::None);
    let mut dragging_point = use_signal(|| Option::<DragTarget>::None);
    let mut active_pick = use_signal(|| Option::<PickTarget>::None);

    let mut x1_input = use_signal(String::new);
    let mut x2_input = use_signal(String::new);
    let mut y1_input = use_signal(String::new);
    let mut y2_input = use_signal(String::new);

    let mut sort_order = use_signal(|| SortOrder::X);
    let mut include_errors = use_signal(|| false);
    let mut connect_lines = use_signal(|| true);
    let marker_size = use_signal(|| 4.5f32);

    let mut auto_trace_step = use_signal(|| 5.0f32);
    let mut auto_trace_tol = use_signal(|| 1.0f32);
    let mut right_tab = use_signal(|| RightTab::Plot);

    let mut x_label = use_signal(|| "X".to_string());
    let mut y_label = use_signal(|| "Y".to_string());

    let mut status_message = use_signal(|| Option::<String>::None);
    let mut status_time = use_signal(|| Option::<Instant>::None);

    let mut selected_point_idx = use_signal(|| Option::<usize>::None);

    let mut is_panning = use_signal(|| false);
    let mut pan_start_mouse = use_signal(|| Point2D { x: 0.0, y: 0.0 });
    let mut pan_start_offset = use_signal(|| Point2D { x: 0.0, y: 0.0 });
    let mut click_start_pos = use_signal(|| Point2D { x: 0.0, y: 0.0 });

    let mut is_dragging = use_signal(|| false);

    let mut left_panel_width = use_signal(|| 320.0f64);
    let mut right_panel_width = use_signal(|| 460.0f64);
    let mut resizing_panel = use_signal(|| Option::<ResizePanel>::None);
    let mut resize_start_x = use_signal(|| 0.0f64);
    let mut resize_start_width = use_signal(|| 0.0f64);

    // Bundle the restorable signals once so both the startup auto-load and
    // the manual "Load Project" action can share `apply_project_data`.
    let project_signals = ProjectSignals {
        image_path,
        image_size,
        image_data_url,
        raw_image,
        calibration,
        points,
        zoom_factor,
        pan_offset,
        sort_order,
        include_errors,
        connect_lines,
        marker_size,
        auto_trace_step,
        auto_trace_tol,
        right_tab,
        x_label,
        y_label,
        x1_input,
        x2_input,
        y1_input,
        y2_input,
    };

    // Auto-load last project or fallback to demo graph on startup
    use_effect(move || {
        let mut loaded = false;
        if let Some(path) = read_last_project_path() {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(data) = serde_json::from_str::<ProjectData>(&json) {
                    let mut sig = project_signals;
                    if let Some(err) = apply_project_data(&mut sig, data) {
                        *status_message.write() = Some(err);
                    } else {
                        *status_message.write() =
                            Some(format!("Auto-loaded last project: {}", path));
                    }
                    *status_time.write() = Some(Instant::now());
                    loaded = true;
                }
            }
        }

        if !loaded {
            let (img, data_url, size) = generate_demo_graph();
            *raw_image.write() = Some(img);
            *image_data_url.write() = Some(data_url);
            *image_size.write() = Some(size);

            let mut cal = Calibration::default();
            cal.x1.pixel = Some(Point2D { x: 100.0, y: 400.0 });
            cal.x1.value = Some(0.0);
            *x1_input.write() = "0.0".to_string();

            cal.x2.pixel = Some(Point2D { x: 500.0, y: 400.0 });
            cal.x2.value = Some(10.0);
            *x2_input.write() = "10.0".to_string();

            cal.y1.pixel = Some(Point2D { x: 100.0, y: 400.0 });
            cal.y1.value = Some(0.0);
            *y1_input.write() = "0.0".to_string();

            cal.y2.pixel = Some(Point2D { x: 100.0, y: 100.0 });
            cal.y2.value = Some(3.0);
            *y2_input.write() = "3.0".to_string();

            *calibration.write() = cal;

            *zoom_factor.write() = 0.8;
            *pan_offset.write() = Point2D { x: 50.0, y: 50.0 };

            *status_message.write() = Some("Loaded interactive sample graph!".to_string());
            *status_time.write() = Some(Instant::now());
        }
    });

    // Helper functions
    let current_status = move || {
        if let Some(msg) = status_message.read().as_ref() {
            if let Some(time) = *status_time.read() {
                if time.elapsed() < Duration::from_secs(5) {
                    return msg.clone();
                }
            }
        }
        "Ready".to_string()
    };

    let open_image = move |_| {
        #[cfg(not(target_os = "android"))]
        spawn(async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
                .pick_file()
                .await;
            if let Some(file) = file {
                let path = file.path().to_string_lossy().to_string();
                match load_image_as_data_url(&path) {
                    Ok((data_url, size, img)) => {
                        *image_path.write() = Some(path.clone());
                        *image_size.write() = Some(size);
                        *image_data_url.write() = Some(data_url);
                        *raw_image.write() = Some(img);

                        *zoom_factor.write() = 0.8;
                        *pan_offset.write() = Point2D { x: 50.0, y: 50.0 };
                        *points.write() = Vec::new();
                        *selected_point_idx.write() = None;

                        *status_message.write() = Some(format!("Loaded image: {}", path));
                        *status_time.write() = Some(Instant::now());
                    }
                    Err(e) => {
                        *status_message.write() = Some(format!("Error loading image: {}", e));
                        *status_time.write() = Some(Instant::now());
                    }
                }
            }
        });
        #[cfg(target_os = "android")]
        {
            *status_message.write() = Some("File picker is not supported on Android".to_string());
            *status_time.write() = Some(Instant::now());
        }
    };

    let load_project = move |_| {
        #[cfg(not(target_os = "android"))]
        spawn(async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("r3data Project", &["r3data"])
                .pick_file()
                .await;
            if let Some(file) = file {
                let path = file.path().to_path_buf();
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(data) = serde_json::from_str::<ProjectData>(&json) {
                        let mut sig = project_signals;
                        if let Some(err) = apply_project_data(&mut sig, data) {
                            *status_message.write() = Some(err);
                        } else {
                            *status_message.write() =
                                Some(format!("Project loaded: {}", path.display()));
                        }
                        *status_time.write() = Some(Instant::now());
                        write_last_project_path(&path.to_string_lossy());
                    }
                }
            }
        });
        #[cfg(target_os = "android")]
        {
            *status_message.write() =
                Some("Loading projects is not supported on Android".to_string());
            *status_time.write() = Some(Instant::now());
        }
    };

    let save_project = move |_| {
        #[cfg(not(target_os = "android"))]
        {
            let path_opt = image_path.read().clone();
            let cal = calibration.read().clone();
            let pts = points.read().clone();
            let xl = x_label.read().clone();
            let yl = y_label.read().clone();
            let zoom = *zoom_factor.read();
            let pan = *pan_offset.read();
            let sort = *sort_order.read();
            let errs = *include_errors.read();
            let conn = *connect_lines.read();
            let m_size = *marker_size.read();
            let trace_step = *auto_trace_step.read();
            let trace_tol = *auto_trace_tol.read();
            let tab = *right_tab.read();
            let x1_in = x1_input.read().clone();
            let x2_in = x2_input.read().clone();
            let y1_in = y1_input.read().clone();
            let y2_in = y2_input.read().clone();

            spawn(async move {
                let file = rfd::AsyncFileDialog::new()
                    .add_filter("r3data Project", &["r3data"])
                    .save_file()
                    .await;
                if let Some(file) = file {
                    let path = file.path().to_path_buf();
                    let data = ProjectData {
                        image_path: path_opt,
                        calibration: cal,
                        points: pts,
                        x_label: xl,
                        y_label: yl,
                        zoom_factor: Some(zoom),
                        pan_offset: Some([pan.x as f32, pan.y as f32]),
                        sort_order: Some(sort),
                        include_errors: Some(errs),
                        connect_lines: Some(conn),
                        marker_size: Some(m_size),
                        auto_trace_step: Some(trace_step),
                        auto_trace_tol: Some(trace_tol),
                        right_tab: Some(tab),
                        x1_input: Some(x1_in),
                        x2_input: Some(x2_in),
                        y1_input: Some(y1_in),
                        y2_input: Some(y2_in),
                    };
                    if let Ok(json) = serde_json::to_string_pretty(&data) {
                        if std::fs::write(&path, json).is_ok() {
                            *status_message.write() =
                                Some(format!("Project saved to {}", path.display()));
                            *status_time.write() = Some(Instant::now());
                            write_last_project_path(&path.to_string_lossy());
                        }
                    }
                }
            });
        }
        #[cfg(target_os = "android")]
        {
            *status_message.write() =
                Some("Saving projects is not supported on Android".to_string());
            *status_time.write() = Some(Instant::now());
        }
    };

    let copy_clipboard = move |_| {
        let text = get_formatted_points(
            &points.read(),
            &calibration.read(),
            *sort_order.read(),
            *include_errors.read(),
        );
        let js = format!(
            "navigator.clipboard.writeText(`{}`);",
            text.replace('`', "\\`").replace('$', "\\$")
        );
        spawn(async move {
            let _ = eval(&js);
            *status_message.write() = Some("Points copied to clipboard!".to_string());
            *status_time.write() = Some(Instant::now());
        });
    };

    let export_csv = move |_| {
        #[cfg(not(target_os = "android"))]
        {
            let text = get_formatted_points(
                &points.read(),
                &calibration.read(),
                *sort_order.read(),
                *include_errors.read(),
            );
            spawn(async move {
                let file = rfd::AsyncFileDialog::new()
                    .add_filter("CSV File", &["csv", "txt"])
                    .save_file()
                    .await;
                if let Some(file) = file {
                    let path = file.path().to_path_buf();
                    if std::fs::write(&path, text).is_ok() {
                        *status_message.write() =
                            Some(format!("Exported points to {}", path.display()));
                        *status_time.write() = Some(Instant::now());
                    }
                }
            });
        }
        #[cfg(target_os = "android")]
        {
            *status_message.write() =
                Some("Exporting to CSV is not supported on Android".to_string());
            *status_time.write() = Some(Instant::now());
        }
    };

    let export_kuva_svg = move |_| {
        #[cfg(not(target_os = "android"))]
        {
            let pts = points.read().clone();
            let cal = calibration.read().clone();
            let xl = x_label.read().clone();
            let yl = y_label.read().clone();
            let sort = *sort_order.read();
            let conn = *connect_lines.read();

            spawn(async move {
                if let Some(svg) = generate_kuva_plot_svg(&pts, &cal, &xl, &yl, sort, conn) {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("SVG Image", &["svg"])
                        .set_file_name("untitled.svg")
                        .save_file()
                        .await;
                    if let Some(file) = file {
                        let path = file.path().to_path_buf();
                        if std::fs::write(&path, svg).is_ok() {
                            *status_message.write() =
                                Some(format!("SVG exported to {}", path.display()));
                            *status_time.write() = Some(Instant::now());
                        }
                    }
                } else {
                    *status_message.write() = Some("No points to export!".to_string());
                    *status_time.write() = Some(Instant::now());
                }
            });
        }
        #[cfg(target_os = "android")]
        {
            *status_message.write() =
                Some("Exporting to SVG is not supported on Android".to_string());
            *status_time.write() = Some(Instant::now());
        }
    };

    let trigger_auto_trace = move |_| {
        let mut pts = points.read().clone();
        let mut sel_idx = selected_point_idx.read().clone();
        let res = trace_all_curves(
            &raw_image.read(),
            &mut pts,
            *auto_trace_step.read(),
            *auto_trace_tol.read(),
            &mut sel_idx,
            &calibration.read(),
        );
        if let Some(msg) = res {
            *points.write() = pts;
            *selected_point_idx.write() = sel_idx;
            *status_message.write() = Some(msg);
            *status_time.write() = Some(Instant::now());
        }
    };

    // Canvas Event Handlers
    let handle_canvas_mousedown = move |event: MouseEvent| {
        let client_coords = event.client_coordinates();
        let cx = client_coords.x - *left_panel_width.read();
        let cy = client_coords.y - 50.0;
        let zoom = *zoom_factor.read();
        let pan = *pan_offset.read();
        let img_x = (cx - pan.x) / zoom as f64;
        let img_y = (cy - pan.y) / zoom as f64;

        let pick_target = *active_pick.read();
        if let Some(target) = pick_target {
            if let Some((w, h)) = *image_size.read() {
                let clamped_pt = Point2D {
                    x: img_x.clamp(0.0, w as f64),
                    y: img_y.clamp(0.0, h as f64),
                };
                let mut cal = calibration.read().clone();
                match target {
                    PickTarget::X1 => cal.x1.pixel = Some(clamped_pt),
                    PickTarget::X2 => cal.x2.pixel = Some(clamped_pt),
                    PickTarget::Y1 => cal.y1.pixel = Some(clamped_pt),
                    PickTarget::Y2 => cal.y2.pixel = Some(clamped_pt),
                }
                *calibration.write() = cal;
                *active_pick.write() = None;
                *status_message.write() = Some(format!("Set calibration point {:?}", target));
                *status_time.write() = Some(Instant::now());
            }
            return;
        }

        let button = event.trigger_button();

        // Right-click: delete point
        if button == Some(MouseButton::Secondary) {
            let mut to_delete = None;
            for (idx, dp) in points.read().iter().enumerate() {
                let scr_x = dp.point.x * zoom as f64 + pan.x + *left_panel_width.read();
                let scr_y = dp.point.y * zoom as f64 + pan.y + 50.0;
                let dist =
                    ((client_coords.x - scr_x).powi(2) + (client_coords.y - scr_y).powi(2)).sqrt();
                if dist < 12.0 {
                    to_delete = Some(idx);
                    break;
                }
            }
            if let Some(idx) = to_delete {
                let mut pts = points.write();
                pts.remove(idx);
                let sel = selected_point_idx.read().clone();
                if sel == Some(idx) {
                    *selected_point_idx.write() = None;
                } else if let Some(s) = sel {
                    if s > idx {
                        *selected_point_idx.write() = Some(s - 1);
                    }
                }
                *status_message.write() = Some("Removed point".to_string());
                *status_time.write() = Some(Instant::now());
            }
            return;
        }

        // Left-click: check drag handles
        if button == Some(MouseButton::Primary) {
            let cal = calibration.read().clone();
            let cps = [
                (cal.x1.pixel, DragTarget::X1),
                (cal.x2.pixel, DragTarget::X2),
                (cal.y1.pixel, DragTarget::Y1),
                (cal.y2.pixel, DragTarget::Y2),
            ];

            let mut found_drag = false;
            for (opt_pt, target) in cps {
                if let Some(pt) = opt_pt {
                    let scr_x = pt.x * zoom as f64 + pan.x + *left_panel_width.read();
                    let scr_y = pt.y * zoom as f64 + pan.y + 50.0;
                    let dist = ((client_coords.x - scr_x).powi(2)
                        + (client_coords.y - scr_y).powi(2))
                    .sqrt();
                    if dist < 15.0 {
                        *dragging_point.write() = Some(target);
                        *is_dragging.write() = true;
                        found_drag = true;
                        break;
                    }
                }
            }

            if !found_drag {
                for (idx, dp) in points.read().iter().enumerate() {
                    let scr_x = dp.point.x * zoom as f64 + pan.x + *left_panel_width.read();
                    let scr_y = dp.point.y * zoom as f64 + pan.y + 50.0;
                    let dist = ((client_coords.x - scr_x).powi(2)
                        + (client_coords.y - scr_y).powi(2))
                    .sqrt();
                    if dist < 12.0 {
                        *dragging_point.write() = Some(DragTarget::DataPoint(idx));
                        *is_dragging.write() = true;
                        found_drag = true;
                        break;
                    }
                }
            }

            if !found_drag {
                *is_panning.write() = true;
                *pan_start_mouse.write() = Point2D {
                    x: client_coords.x,
                    y: client_coords.y,
                };
                *pan_start_offset.write() = pan;
            }
            *click_start_pos.write() = Point2D {
                x: client_coords.x,
                y: client_coords.y,
            };
        } else if button == Some(MouseButton::Auxiliary) {
            *is_panning.write() = true;
            *pan_start_mouse.write() = Point2D {
                x: client_coords.x,
                y: client_coords.y,
            };
            *pan_start_offset.write() = pan;
        }
    };

    let handle_canvas_mousemove = move |event: MouseEvent| {
        let client_coords = event.client_coordinates();
        let cx = client_coords.x - *left_panel_width.read();
        let cy = client_coords.y - 50.0;
        let zoom = *zoom_factor.read();
        let pan = *pan_offset.read();
        let img_x = (cx - pan.x) / zoom as f64;
        let img_y = (cy - pan.y) / zoom as f64;

        if let Some((w, h)) = *image_size.read() {
            if img_x >= 0.0 && img_x <= w as f64 && img_y >= 0.0 && img_y <= h as f64 {
                *hovered_pixel.write() = Some(Point2D { x: img_x, y: img_y });
            } else {
                *hovered_pixel.write() = None;
            }
        } else {
            *hovered_pixel.write() = None;
        }

        if *is_panning.read() {
            let start_mouse = *pan_start_mouse.read();
            let start_offset = *pan_start_offset.read();
            let dx = client_coords.x - start_mouse.x;
            let dy = client_coords.y - start_mouse.y;
            *pan_offset.write() = Point2D {
                x: start_offset.x + dx,
                y: start_offset.y + dy,
            };
        }

        if let Some(target) = *dragging_point.read() {
            if let Some((w, h)) = *image_size.read() {
                let clamped_pt = Point2D {
                    x: img_x.clamp(0.0, w as f64),
                    y: img_y.clamp(0.0, h as f64),
                };
                match target {
                    DragTarget::X1 => {
                        let mut cal = calibration.write();
                        cal.x1.pixel = Some(clamped_pt);
                    }
                    DragTarget::X2 => {
                        let mut cal = calibration.write();
                        cal.x2.pixel = Some(clamped_pt);
                    }
                    DragTarget::Y1 => {
                        let mut cal = calibration.write();
                        cal.y1.pixel = Some(clamped_pt);
                    }
                    DragTarget::Y2 => {
                        let mut cal = calibration.write();
                        cal.y2.pixel = Some(clamped_pt);
                    }
                    DragTarget::DataPoint(idx) => {
                        let mut pts = points.write();
                        if idx < pts.len() {
                            pts[idx].point = clamped_pt;
                        }
                    }
                }
            }
        }
    };

    let handle_canvas_mouseup = move |event: MouseEvent| {
        let client_coords = event.client_coordinates();
        let start_pos = *click_start_pos.read();
        let dist = ((client_coords.x - start_pos.x).powi(2)
            + (client_coords.y - start_pos.y).powi(2))
        .sqrt();

        *is_panning.write() = false;
        *is_dragging.write() = false;
        *dragging_point.write() = None;

        if dist < 5.0 && event.trigger_button() == Some(MouseButton::Primary) {
            let cx = client_coords.x - *left_panel_width.read();
            let cy = client_coords.y - 50.0;
            let zoom = *zoom_factor.read();
            let pan = *pan_offset.read();
            let img_x = (cx - pan.x) / zoom as f64;
            let img_y = (cy - pan.y) / zoom as f64;

            if let Some((w, h)) = *image_size.read() {
                if img_x >= 0.0 && img_x <= w as f64 && img_y >= 0.0 && img_y <= h as f64 {
                    let mut close_to_cp = false;
                    let cal = calibration.read().clone();
                    let cps = [cal.x1.pixel, cal.x2.pixel, cal.y1.pixel, cal.y2.pixel];
                    for cp in cps.iter().flatten() {
                        let scr_x = cp.x * zoom as f64 + pan.x + *left_panel_width.read();
                        let scr_y = cp.y * zoom as f64 + pan.y + 50.0;
                        let d = ((client_coords.x - scr_x).powi(2)
                            + (client_coords.y - scr_y).powi(2))
                        .sqrt();
                        if d < 15.0 {
                            close_to_cp = true;
                            break;
                        }
                    }

                    if !close_to_cp {
                        let mut clicked_dp_idx = None;
                        for (idx, dp) in points.read().iter().enumerate() {
                            let scr_x = dp.point.x * zoom as f64 + pan.x + *left_panel_width.read();
                            let scr_y = dp.point.y * zoom as f64 + pan.y + 50.0;
                            let d = ((client_coords.x - scr_x).powi(2)
                                + (client_coords.y - scr_y).powi(2))
                            .sqrt();
                            if d < 12.0 {
                                clicked_dp_idx = Some(idx);
                                break;
                            }
                        }

                        if let Some(idx) = clicked_dp_idx {
                            *selected_point_idx.write() = Some(idx);
                            *status_message.write() = Some(format!("Selected point #{}", idx + 1));
                            *status_time.write() = Some(Instant::now());
                        } else {
                            let new_pt = Point2D { x: img_x, y: img_y };
                            let mut pts = points.write();
                            pts.push(DataPoint {
                                point: new_pt,
                                source: PointSource::Manual,
                            });
                            *selected_point_idx.write() = Some(pts.len() - 1);
                        }
                    }
                }
            }
        }
    };

    let handle_wheel = move |event: WheelEvent| {
        let delta = event.delta().strip_units().y;
        if delta == 0.0 {
            return;
        }

        let zoom_multiplier = (-delta * 0.0015).exp();
        let old_zoom = *zoom_factor.read();
        let new_zoom = (old_zoom * zoom_multiplier as f32).clamp(0.05, 50.0);

        let client_pos = event.client_coordinates();
        let cx = client_pos.x - *left_panel_width.read();
        let cy = client_pos.y - 50.0;

        let pan = *pan_offset.read();
        let new_pan = Point2D {
            x: cx - (cx - pan.x) * (new_zoom / old_zoom) as f64,
            y: cy - (cy - pan.y) * (new_zoom / old_zoom) as f64,
        };

        *zoom_factor.write() = new_zoom;
        *pan_offset.write() = new_pan;
    };

    // Side panel splitter drag handlers
    let start_resize_left = move |event: MouseEvent| {
        event.stop_propagation();
        *resizing_panel.write() = Some(ResizePanel::Left);
        *resize_start_x.write() = event.client_coordinates().x;
        *resize_start_width.write() = *left_panel_width.read();
    };

    let start_resize_right = move |event: MouseEvent| {
        event.stop_propagation();
        *resizing_panel.write() = Some(ResizePanel::Right);
        *resize_start_x.write() = event.client_coordinates().x;
        *resize_start_width.write() = *right_panel_width.read();
    };

    let handle_workspace_mousemove = move |event: MouseEvent| {
        if let Some(side) = *resizing_panel.read() {
            let dx = event.client_coordinates().x - *resize_start_x.read();
            match side {
                ResizePanel::Left => {
                    let new_w = (*resize_start_width.read() + dx)
                        .clamp(LEFT_PANEL_MIN_WIDTH, LEFT_PANEL_MAX_WIDTH);
                    *left_panel_width.write() = new_w;
                }
                ResizePanel::Right => {
                    let new_w = (*resize_start_width.read() - dx)
                        .clamp(RIGHT_PANEL_MIN_WIDTH, RIGHT_PANEL_MAX_WIDTH);
                    *right_panel_width.write() = new_w;
                }
            }
        }
    };

    let handle_workspace_mouseup = move |_event: MouseEvent| {
        *resizing_panel.write() = None;
    };

    // Form value changes
    let handle_x1_change = move |e: FormEvent| {
        let val_str = e.value();
        *x1_input.write() = val_str.clone();
        if let Ok(v) = val_str.trim().parse::<f64>() {
            let mut cal = calibration.write();
            cal.x1.value = Some(v);
        }
    };

    // Form value changes
    let handle_x2_change = move |e: FormEvent| {
        let val_str = e.value();
        *x2_input.write() = val_str.clone();
        if let Ok(v) = val_str.trim().parse::<f64>() {
            let mut cal = calibration.write();
            cal.x2.value = Some(v);
        }
    };

    let handle_y1_change = move |e: FormEvent| {
        let val_str = e.value();
        *y1_input.write() = val_str.clone();
        if let Ok(v) = val_str.trim().parse::<f64>() {
            let mut cal = calibration.write();
            cal.y1.value = Some(v);
        }
    };

    let handle_y2_change = move |e: FormEvent| {
        let val_str = e.value();
        *y2_input.write() = val_str.clone();
        if let Ok(v) = val_str.trim().parse::<f64>() {
            let mut cal = calibration.write();
            cal.y2.value = Some(v);
        }
    };

    // View calculations
    let zoom = *zoom_factor.read();
    let pan = *pan_offset.read();
    let points_val = points.read();
    let cal_val = calibration.read();

    // Render connecting path SVG
    let mut path_d = String::new();
    for (idx, dp) in points_val.iter().enumerate() {
        let cmd = if idx == 0 { "M" } else { "L" };
        path_d.push_str(&format!("{} {} {} ", cmd, dp.point.x, dp.point.y));
    }

    // Generate dynamic Kuva SVG plot preview
    let live_plot_svg = {
        let xl = x_label.read();
        let yl = y_label.read();
        let sort = *sort_order.read();
        let conn = *connect_lines.read();
        generate_kuva_plot_svg(&points_val, &cal_val, &xl, &yl, sort, conn)
    };

    rsx! {
        style { {include_str!("style.css")} }
        div {
            class: "app-container",
            // Header Topbar
            header {
                class: "topbar",
                div {
                    class: "brand",
                    "📈 r3data"
                    span { class: "brand-accent", "Desktop" }
                }
                div {
                    class: "menu-actions",
                    button { onclick: open_image, "📂 Open Image..." }
                    button { onclick: load_project, "📤 Load Project..." }
                    button { onclick: save_project, "📥 Save Project..." }
                    button { onclick: copy_clipboard, "📋 Copy Clipboard" }
                    button { onclick: export_csv, "💾 Export CSV..." }
                }
            }

            // Main Columns
            main {
                class: "main-workspace",
                onmousemove: handle_workspace_mousemove,
                onmouseup: handle_workspace_mouseup,
                onmouseleave: handle_workspace_mouseup,

                // Left Panel: Magnifier and Calibration controls
                div {
                    class: "left-panel",
                    style: "width: {left_panel_width}px;",

                    // Magnifier Viewport
                    div {
                        class: "magnifier-section",
                        span { class: "magnifier-title", "🔍 Magnifier" }
                        div {
                            class: "magnifier-viewport",
                            if let (Some(data_url), Some(hov_pt)) = (image_data_url.read().as_ref(), *hovered_pixel.read()) {
                                if let Some((img_w, img_h)) = *image_size.read() {
                                    div {
                                        style: "width: 100%; height: 100%; background-image: url({data_url}); background-size: {img_w * 8}px {img_h * 8}px; background-position: {144.0 - hov_pt.x * 8.0}px {90.0 - hov_pt.y * 8.0}px; background-repeat: no-repeat;"
                                    }
                                } else {
                                    div {
                                        style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; color: var(--text-muted); font-size: 0.8rem;",
                                        "Hover graph to magnify"
                                    }
                                }
                            } else {
                                div {
                                    style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; color: var(--text-muted); font-size: 0.8rem;",
                                    "Hover graph to magnify"
                                }
                            }
                            div { class: "magnifier-crosshair" }
                        }
                    }

                    // Axis Calibration Panel
                    div {
                        class: "panel-group",
                        div {
                            class: "panel-title",
                            "📐 Axis Calibration"
                        }
                        div {
                            class: "calibration-grid",

                            span { class: "calibration-label", "X1 value" }
                            input {
                                r#type: "text",
                                value: "{x1_input}",
                                oninput: handle_x1_change,
                            }
                            button {
                                class: if *active_pick.read() == Some(PickTarget::X1) { "toggle-active" } else { "" },
                                onclick: move |_| {
                                    let active = *active_pick.read() == Some(PickTarget::X1);
                                    *active_pick.write() = if active { None } else { Some(PickTarget::X1) };
                                },
                                "📍 Pick"
                            }

                            span { class: "calibration-label", "X2 value" }
                            input {
                                r#type: "text",
                                value: "{x2_input}",
                                oninput: handle_x2_change,
                            }
                            button {
                                class: if *active_pick.read() == Some(PickTarget::X2) { "toggle-active" } else { "" },
                                onclick: move |_| {
                                    let active = *active_pick.read() == Some(PickTarget::X2);
                                    *active_pick.write() = if active { None } else { Some(PickTarget::X2) };
                                },
                                "📍 Pick"
                            }

                            span { class: "calibration-label", "Y1 value" }
                            input {
                                r#type: "text",
                                value: "{y1_input}",
                                oninput: handle_y1_change,
                            }
                            button {
                                class: if *active_pick.read() == Some(PickTarget::Y1) { "toggle-active" } else { "" },
                                onclick: move |_| {
                                    let active = *active_pick.read() == Some(PickTarget::Y1);
                                    *active_pick.write() = if active { None } else { Some(PickTarget::Y1) };
                                },
                                "📍 Pick"
                            }

                            span { class: "calibration-label", "Y2 value" }
                            input {
                                r#type: "text",
                                value: "{y2_input}",
                                oninput: handle_y2_change,
                            }
                            button {
                                class: if *active_pick.read() == Some(PickTarget::Y2) { "toggle-active" } else { "" },
                                onclick: move |_| {
                                    let active = *active_pick.read() == Some(PickTarget::Y2);
                                    *active_pick.write() = if active { None } else { Some(PickTarget::Y2) };
                                },
                                "📍 Pick"
                            }
                        }

                        div {
                            class: "checkbox-row",
                            label {
                                input {
                                    r#type: "checkbox",
                                    checked: cal_val.x_log,
                                    onchange: move |e| {
                                        let mut cal = calibration.write();
                                        cal.x_log = e.value().parse::<bool>().unwrap_or(false);
                                    }
                                }
                                "X Log"
                            }
                            label {
                                input {
                                    r#type: "checkbox",
                                    checked: cal_val.y_log,
                                    onchange: move |e| {
                                        let mut cal = calibration.write();
                                        cal.y_log = e.value().parse::<bool>().unwrap_or(false);
                                    }
                                }
                                "Y Log"
                            }
                        }

                        div {
                            class: "input-row",
                            label { "X Axis Label:" }
                            input {
                                r#type: "text",
                                value: "{x_label}",
                                oninput: move |e| *x_label.write() = e.value(),
                            }
                        }
                        div {
                            class: "input-row",
                            label { "Y Axis Label:" }
                            input {
                                r#type: "text",
                                value: "{y_label}",
                                oninput: move |e| *y_label.write() = e.value(),
                            }
                        }
                    }

                    // Auto-Trace Settings Panel
                    div {
                        class: "panel-group",
                        div {
                            class: "panel-title",
                            "✨ Auto Trace Curve"
                        }

                        div {
                            class: "slider-group",
                            div {
                                class: "slider-header",
                                span { "Step (px)" }
                                span { class: "slider-value", "{auto_trace_step}" }
                            }
                            input {
                                r#type: "range",
                                min: "5",
                                max: "50",
                                step: "1",
                                value: "{auto_trace_step}",
                                oninput: move |e| *auto_trace_step.write() = e.value().parse::<f32>().unwrap_or(5.0),
                            }
                        }

                        div {
                            class: "slider-group",
                            div {
                                class: "slider-header",
                                span { "Tolerance" }
                                span { class: "slider-value", "{auto_trace_tol}" }
                            }
                            input {
                                r#type: "range",
                                min: "1",
                                max: "20",
                                step: "0.5",
                                value: "{auto_trace_tol}",
                                oninput: move |e| *auto_trace_tol.write() = e.value().parse::<f32>().unwrap_or(1.0),
                            }
                        }

                        button {
                            class: "primary",
                            onclick: trigger_auto_trace,
                            "⚡ Trace Curve"
                        }

                        // Trace guide notifications
                        {
                            let manual_count = points_val.iter().filter(|dp| dp.source == PointSource::Manual).count();
                            let single_manual_selected = if let Some(idx) = *selected_point_idx.read() {
                                idx < points_val.len() && points_val[idx].source == PointSource::Manual
                            } else {
                                false
                            };

                            if manual_count >= 2 {
                                rsx! {
                                    span {
                                        class: "status-badge status-ready",
                                        "Found {manual_count} manual guides. Ready to trace between them."
                                    }
                                }
                            } else if single_manual_selected {
                                rsx! {
                                    span {
                                        class: "status-badge status-ready",
                                        "Point #{selected_point_idx.read().unwrap() + 1} selected. Ready to trace rightward."
                                    }
                                }
                            } else {
                                rsx! {
                                    span {
                                        class: "status-badge status-warning",
                                        "Place 2+ manual points (or select 1 manual point) to enable auto tracing"
                                    }
                                }
                            }
                        }
                    }
                }

                // Draggable splitter between the left panel and the canvas
                div {
                    class: "panel-divider",
                    onmousedown: start_resize_left,
                }

                // Center Column: Interactive canvas viewport
                div {
                    class: "canvas-column",

                    // Canvas container wrapper
                    div {
                        class: "canvas-container",
                        onmousedown: handle_canvas_mousedown,
                        onmousemove: handle_canvas_mousemove,
                        onmouseup: handle_canvas_mouseup,
                        onwheel: handle_wheel,

                        // Transformed Viewport
                        div {
                            class: "canvas-viewport",
                            style: "transform: translate({pan.x}px, {pan.y}px) scale({zoom});",

                            // Raw Graph Image
                            {
                                if let (Some(data_url), Some((img_w, img_h))) = (image_data_url.read().as_ref(), *image_size.read()) {
                                    rsx! {
                                        img {
                                            class: "canvas-image",
                                            src: "{data_url}",
                                            style: "width: {img_w}px; height: {img_h}px;",
                                        }

                                        // SVG Line Overlay
                                        svg {
                                            class: "canvas-svg-layer",
                                            style: "width: {img_w}px; height: {img_h}px; position: absolute; left: 0; top: 0;",

                                            // Calibration line X1 to X2
                                            if let (Some(x1), Some(x2)) = (cal_val.x1.pixel, cal_val.x2.pixel) {
                                                line {
                                                    x1: "{x1.x}",
                                                    y1: "{x1.y}",
                                                    x2: "{x2.x}",
                                                    y2: "{x2.y}",
                                                    stroke: "rgba(56, 189, 248, 0.4)",
                                                    stroke_width: "{1.2 / zoom}",
                                                    stroke_dasharray: "{5.0 / zoom},{5.0 / zoom}",
                                                }
                                            }

                                            // Calibration line Y1 to Y2
                                            if let (Some(y1), Some(y2)) = (cal_val.y1.pixel, cal_val.y2.pixel) {
                                                line {
                                                    x1: "{y1.x}",
                                                    y1: "{y1.y}",
                                                    x2: "{y2.x}",
                                                    y2: "{y2.y}",
                                                    stroke: "rgba(16, 185, 129, 0.4)",
                                                    stroke_width: "{1.2 / zoom}",
                                                    stroke_dasharray: "{5.0 / zoom},{5.0 / zoom}",
                                                }
                                            }

                                            // Data Point Connecting Lines
                                            if *connect_lines.read() && !path_d.is_empty() {
                                                path {
                                                    d: "{path_d}",
                                                    fill: "none",
                                                    stroke: "rgba(127, 140, 141, 0.6)",
                                                    stroke_width: "{1.5 / zoom}",
                                                }
                                            }
                                        }

                                        // Calibration markers
                                        if let Some(pt) = cal_val.x1.pixel {
                                            div {
                                                class: "calibration-marker x",
                                                style: "left: {pt.x}px; top: {pt.y}px; transform: translate(-50%, -50%) scale({1.0 / zoom});",
                                                span { class: "marker-label", "X1" }
                                            }
                                        }
                                        if let Some(pt) = cal_val.x2.pixel {
                                            div {
                                                class: "calibration-marker x",
                                                style: "left: {pt.x}px; top: {pt.y}px; transform: translate(-50%, -50%) scale({1.0 / zoom});",
                                                span { class: "marker-label", "X2" }
                                            }
                                        }
                                        if let Some(pt) = cal_val.y1.pixel {
                                            div {
                                                class: "calibration-marker y",
                                                style: "left: {pt.x}px; top: {pt.y}px; transform: translate(-50%, -50%) scale({1.0 / zoom});",
                                                span { class: "marker-label", "Y1" }
                                            }
                                        }
                                        if let Some(pt) = cal_val.y2.pixel {
                                            div {
                                                class: "calibration-marker y",
                                                style: "left: {pt.x}px; top: {pt.y}px; transform: translate(-50%, -50%) scale({1.0 / zoom});",
                                                span { class: "marker-label", "Y2" }
                                            }
                                        }

                                        // Data point markers
                                        {
                                            points_val.iter().enumerate().map(|(idx, dp)| {
                                                let pt = dp.point;
                                                let is_selected = *selected_point_idx.read() == Some(idx);
                                                let class = if dp.source == PointSource::Manual { "data-point-marker manual" } else { "data-point-marker autotrace" };
                                                let select_class = if is_selected { "selected" } else { "" };
                                                let tol = *auto_trace_tol.read();

                                                rsx! {
                                                    div {
                                                        key: "{idx}",
                                                        class: "{class} {select_class}",
                                                        style: "left: {pt.x}px; top: {pt.y}px; transform: translate(-50%, -50%) scale({1.0 / zoom});",
                                                        span { class: "data-point-label", "{idx + 1}" }

                                                        // Draw tolerance ring around selected manual guide point
                                                        if is_selected && dp.source == PointSource::Manual {
                                                            div {
                                                                class: "data-point-tolerance-ring",
                                                                style: "left: 4px; top: 4px; width: {tol * 2.0}px; height: {tol * 2.0}px;"
                                                            }
                                                        }
                                                    }
                                                }
                                            })
                                        }
                                    }
                                } else {
                                    rsx! { "" }
                                }
                            }
                        }
                    }

                    // Interactive target pick display warnings
                    if let Some(target) = *active_pick.read() {
                        div {
                            class: "hud-pick-overlay",
                            "📍 Click on graph to place {target:?}"
                        }
                    }
                }

                // Draggable splitter between the canvas and the right panel
                div {
                    class: "panel-divider",
                    onmousedown: start_resize_right,
                }

                // Right Panel: Data Tab sheets (Table, Live Plot, and Kuva Plots)
                div {
                    class: "right-panel",
                    style: "width: {right_panel_width}px;",

                    // Tab selections
                    div {
                        class: "tab-header",
                        button {
                            class: if *right_tab.read() == RightTab::Points { "tab-btn active" } else { "tab-btn" },
                            onclick: move |_| *right_tab.write() = RightTab::Points,
                            "🎯 Points Data"
                        }
                        button {
                            class: if *right_tab.read() == RightTab::Plot { "tab-btn active" } else { "tab-btn" },
                            onclick: move |_| *right_tab.write() = RightTab::Plot,
                            "📊 Live Plot"
                        }
                        button {
                            class: if *right_tab.read() == RightTab::KuvaPlot { "tab-btn active" } else { "tab-btn" },
                            onclick: move |_| *right_tab.write() = RightTab::KuvaPlot,
                            "🖼 Kuva Plot"
                        }
                    }

                    // Tab sheets content
                    div {
                        class: "tab-content",

                        match *right_tab.read() {
                            RightTab::Points => rsx! {
                                div {
                                    class: "points-list-header",
                                    h3 { "🎯 Points: {points_val.len()}" }
                                    div {
                                        style: "display: flex; gap: 8px;",
                                        button {
                                            onclick: move |_| {
                                                let mut pts = points.write();
                                                pts.pop();
                                                let sel = *selected_point_idx.read();
                                                if let Some(s) = sel {
                                                    if s >= pts.len() {
                                                        *selected_point_idx.write() = None;
                                                    }
                                                }
                                            },
                                            "↩ Undo"
                                        }
                                        button {
                                            class: "danger",
                                            onclick: move |_| {
                                                points.write().clear();
                                                *selected_point_idx.write() = None;
                                            },
                                            "🗑 Clear"
                                        }
                                    }
                                }

                                div {
                                    class: "points-scroll-area",
                                    table {
                                        class: "points-table",
                                        thead {
                                            tr {
                                                th { "Index" }
                                                th { "Type" }
                                                th { "Coordinates (X, Y)" }
                                                th { style: "width: 40px;" }
                                            }
                                        }
                                        tbody {
                                            {
                                                points_val.iter().enumerate().map(|(idx, dp)| {
                                                    let is_sel = *selected_point_idx.read() == Some(idx);
                                                    let row_sel_class = if is_sel { "selected" } else { "" };
                                                    let (badge_char, badge_class) = match dp.source {
                                                        PointSource::Manual => ("📍 Manual", "point-source-badge point-source-manual"),
                                                        PointSource::AutoTrace => ("⚡ Auto", "point-source-badge point-source-auto"),
                                                    };

                                                    rsx! {
                                                        tr {
                                                            key: "{idx}",
                                                            class: "{row_sel_class} point-row-clickable",
                                                            onclick: move |_| *selected_point_idx.write() = Some(idx),

                                                            td { "{idx + 1}" }
                                                            td { span { class: "{badge_class}", "{badge_char}" } }
                                                            td {
                                                                {
                                                                    if let Some(cal) = cal_val.calculate(dp.point) {
                                                                        if *include_errors.read() {
                                                                            format!("({:.3}±{:.3}, {:.3}±{:.3})", cal.x_val, cal.x_err, cal.y_val, cal.y_err)
                                                                        } else {
                                                                            format!("({:.3}, {:.3})", cal.x_val, cal.y_val)
                                                                        }
                                                                    } else {
                                                                        format!("px({:.0}, {:.0})", dp.point.x, dp.point.y)
                                                                    }
                                                                }
                                                            }
                                                            td {
                                                                button {
                                                                    class: "danger",
                                                                    style: "padding: 2px 6px; font-size: 0.75rem;",
                                                                    onclick: move |e| {
                                                                        e.stop_propagation();
                                                                        let mut pts = points.write();
                                                                        pts.remove(idx);
                                                                        let sel = selected_point_idx.read().clone();
                                                                        if sel == Some(idx) {
                                                                            *selected_point_idx.write() = None;
                                                                        } else if let Some(s) = sel {
                                                                            if s > idx {
                                                                                *selected_point_idx.write() = Some(s - 1);
                                                                            }
                                                                        }
                                                                    },
                                                                    "❌"
                                                                }
                                                            }
                                                        }
                                                    }
                                                })
                                            }
                                        }
                                    }
                                }

                                div {
                                    class: "panel-group",
                                    div { class: "panel-title", "⚙ Output Configuration" }

                                    div {
                                        class: "checkbox-row",
                                        span { "Sort:" }
                                        label {
                                            input {
                                                r#type: "radio",
                                                name: "sort_order",
                                                checked: *sort_order.read() == SortOrder::None,
                                                onchange: move |_| *sort_order.write() = SortOrder::None,
                                            }
                                            "None"
                                        }
                                        label {
                                            input {
                                                r#type: "radio",
                                                name: "sort_order",
                                                checked: *sort_order.read() == SortOrder::X,
                                                onchange: move |_| *sort_order.write() = SortOrder::X,
                                            }
                                            "X"
                                        }
                                        label {
                                            input {
                                                r#type: "radio",
                                                name: "sort_order",
                                                checked: *sort_order.read() == SortOrder::Y,
                                                onchange: move |_| *sort_order.write() = SortOrder::Y,
                                            }
                                            "Y"
                                        }
                                    }

                                    label {
                                        class: "checkbox-row",
                                        input {
                                            r#type: "checkbox",
                                            checked: *include_errors.read(),
                                            onchange: move |e| *include_errors.write() = e.value().parse::<bool>().unwrap_or(false),
                                        }
                                        "Include estimation errors"
                                    }
                                }
                            },
                            RightTab::Plot => rsx! {
                                div {
                                    class: "plot-controls",
                                    label {
                                        class: "checkbox-row",
                                        input {
                                            r#type: "checkbox",
                                            checked: *connect_lines.read(),
                                            onchange: move |e| *connect_lines.write() = e.value().parse::<bool>().unwrap_or(true),
                                        }
                                        "Connect path lines"
                                    }
                                }

                                if let Some(svg) = live_plot_svg {
                                    div {
                                        class: "plot-container-dark",
                                        dangerous_inner_html: "{svg}",
                                    }
                                } else {
                                    div {
                                        class: "status-badge status-warning",
                                        style: "text-align: center; padding: 24px;",
                                        "Configure calibration and place points to generate plot"
                                    }
                                }
                            },
                            RightTab::KuvaPlot => rsx! {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 16px; align-items: center;",
                                    h3 { "🖼 Kuva SVG Export Profile" }

                                    button {
                                        class: "primary",
                                        onclick: export_kuva_svg,
                                        "Export to SVG..."
                                    }

                                    if let Some(svg) = live_plot_svg {
                                        div {
                                            class: "plot-container",
                                            style: "background-color: white;",
                                            dangerous_inner_html: "{svg}",
                                        }
                                    } else {
                                        div {
                                            class: "status-badge status-warning",
                                            style: "text-align: center; padding: 24px; width: 100%;",
                                            "No points to plot"
                                        }
                                    }

                                    p {
                                        style: "font-size: 0.8rem; color: var(--text-secondary); text-align: center; line-height: 1.4;",
                                        "Generates a publication-quality vector graphic using the Kuva layout rendering engine, configuring strict range borders and symmetrical paddings."
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Footer Statusbar
            footer {
                class: "statusbar",
                span {
                    class: "status-message-info",
                    "{current_status()}"
                }

                div {
                    class: "status-coord-info",
                    {
                        if let Some(pt) = *hovered_pixel.read() {
                            let mut txt = format!("Pixel: ({:.1}, {:.1})", pt.x, pt.y);
                            if let Some(cal) = cal_val.calculate(pt) {
                                txt += &format!(" | Graph: ({:.5}, {:.5})", cal.x_val, cal.y_val);
                                if *include_errors.read() {
                                    txt += &format!(" ± ({:.5}, {:.5})", cal.x_err, cal.y_err);
                                }
                            }
                            txt
                        } else {
                            "Pointer outside graph".to_string()
                        }
                    }
                }
            }
        }
    }
}
