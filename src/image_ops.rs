use crate::calibration::{Calibration, Point2D};
use crate::model::{DataPoint, PointSource, SortOrder};
use base64::Engine;

pub fn load_image_as_data_url(
    path: &str,
) -> Result<(String, (u32, u32), image::DynamicImage), String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let width = img.width();
    let height = img.height();

    let mut bytes = std::io::Cursor::new(Vec::new());
    img.write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes.into_inner());
    let data_url = format!("data:image/png;base64,{}", b64);

    Ok((data_url, (width, height), img))
}

pub fn generate_demo_graph() -> (image::DynamicImage, String, (u32, u32)) {
    let width = 600;
    let height = 500;
    let mut imgbuf =
        image::ImageBuffer::from_pixel(width, height, image::Rgba([250, 250, 245, 255]));

    // Draw grid lines
    for x in (50..width).step_by(50) {
        for y in 50..(height - 50) {
            imgbuf.put_pixel(x, y, image::Rgba([230, 230, 225, 255]));
        }
    }
    for y in (50..height - 50).step_by(50) {
        for x in 50..(width - 50) {
            imgbuf.put_pixel(x, y, image::Rgba([230, 230, 225, 255]));
        }
    }

    // Draw axes (X = 400, Y = 100)
    for x in 50..(width - 50) {
        for dy in -1..=1 {
            imgbuf.put_pixel(x, (400 + dy) as u32, image::Rgba([80, 80, 80, 255]));
        }
    }
    for y in 50..(height - 50) {
        for dx in -1..=1 {
            imgbuf.put_pixel((100 + dx) as u32, y, image::Rgba([80, 80, 80, 255]));
        }
    }

    // Draw some tick marks
    for x in (100..=(width - 100)).step_by(100) {
        for dy in -5..=5 {
            imgbuf.put_pixel(x as u32, (400 + dy) as u32, image::Rgba([40, 40, 40, 255]));
        }
    }
    for y in (100..=400).step_by(100) {
        for dx in -5..=5 {
            imgbuf.put_pixel((100 + dx) as u32, y as u32, image::Rgba([40, 40, 40, 255]));
        }
    }

    // Draw mathematical curves (a nice sine curve)
    for x in 100..(width - 100) {
        let x_val = (x - 100) as f64 / 40.0;
        let y_val = (x_val.sin() + 1.0) * 100.0;
        let y_pixel = 400 - y_val as i32;
        if y_pixel >= 50 && y_pixel < (height as i32 - 50) {
            for dx in -2..=2 {
                for dy in -2..=2 {
                    let px = (x as i32 + dx) as u32;
                    let py = (y_pixel + dy) as u32;
                    if px < width && py < height {
                        imgbuf.put_pixel(px, py, image::Rgba([41, 128, 185, 255])); // Blue line
                    }
                }
            }
        }
    }

    let dynamic_image = image::DynamicImage::ImageRgba8(imgbuf);

    // Save to base64
    let mut bytes = std::io::Cursor::new(Vec::new());
    let _ = dynamic_image.write_to(&mut bytes, image::ImageFormat::Png);
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes.into_inner());
    let data_url = format!("data:image/png;base64,{}", b64);

    (dynamic_image, data_url, (width, height))
}

pub fn generate_kuva_plot_svg(
    points: &[DataPoint],
    calibration: &Calibration,
    x_label: &str,
    y_label: &str,
    sort_order: SortOrder,
    connect_lines: bool,
) -> Option<String> {
    let mut pts = points.to_vec();
    match sort_order {
        SortOrder::None => {}
        SortOrder::X => {
            pts.sort_by(|a, b| {
                let val_a = calibration
                    .calculate(a.point)
                    .map(|v| v.x_val)
                    .unwrap_or(0.0);
                let val_b = calibration
                    .calculate(b.point)
                    .map(|v| v.x_val)
                    .unwrap_or(0.0);
                val_a
                    .partial_cmp(&val_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortOrder::Y => {
            pts.sort_by(|a, b| {
                let val_a = calibration
                    .calculate(a.point)
                    .map(|v| v.y_val)
                    .unwrap_or(0.0);
                let val_b = calibration
                    .calculate(b.point)
                    .map(|v| v.y_val)
                    .unwrap_or(0.0);
                val_a
                    .partial_cmp(&val_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    let mut plot_data = Vec::new();
    for dp in &pts {
        if let Some(cal) = calibration.calculate(dp.point) {
            plot_data.push((cal.x_val, cal.y_val));
        }
    }

    if plot_data.is_empty() {
        return None;
    }

    let mut plots = Vec::new();

    if connect_lines {
        let line = kuva::plot::LinePlot::new()
            .with_data(plot_data.clone())
            .with_color("steelblue")
            .with_stroke_width(2.0);
        plots.push(kuva::render::plots::Plot::Line(line));
    }

    let scatter = kuva::plot::scatter::ScatterPlot::new()
        .with_data(plot_data)
        .with_color("steelblue")
        .with_size(5.0);
    plots.push(kuva::render::plots::Plot::Scatter(scatter));

    let layout = kuva::render::layout::Layout::auto_from_plots(&plots)
        .with_x_label(x_label)
        .with_y_label(y_label);

    let scene = kuva::render::render::render_multiple(plots, layout);
    use kuva::backend::svg::SvgBackend;
    let svg = SvgBackend.render_scene(&scene);

    Some(svg)
}

pub fn get_formatted_points(
    points: &[DataPoint],
    calibration: &Calibration,
    sort_order: SortOrder,
    include_errors: bool,
) -> String {
    let mut out = String::new();
    let mut pts = points.to_vec();

    match sort_order {
        SortOrder::None => {}
        SortOrder::X => {
            pts.sort_by(|a, b| {
                let val_a = calibration
                    .calculate(a.point)
                    .map(|v| v.x_val)
                    .unwrap_or(0.0);
                let val_b = calibration
                    .calculate(b.point)
                    .map(|v| v.x_val)
                    .unwrap_or(0.0);
                val_a
                    .partial_cmp(&val_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortOrder::Y => {
            pts.sort_by(|a, b| {
                let val_a = calibration
                    .calculate(a.point)
                    .map(|v| v.y_val)
                    .unwrap_or(0.0);
                let val_b = calibration
                    .calculate(b.point)
                    .map(|v| v.y_val)
                    .unwrap_or(0.0);
                val_a
                    .partial_cmp(&val_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    for pt in pts {
        if let Some(cal) = calibration.calculate(pt.point) {
            let source_str = match pt.source {
                PointSource::Manual => "Manual",
                PointSource::AutoTrace => "AutoTrace",
            };
            if include_errors {
                out.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\n",
                    cal.x_val, cal.y_val, cal.x_err, cal.y_err, source_str
                ));
            } else {
                out.push_str(&format!("{}\t{}\t{}\n", cal.x_val, cal.y_val, source_str));
            }
        }
    }
    out
}

pub fn trace_all_curves(
    raw_image: &Option<image::DynamicImage>,
    points: &mut Vec<DataPoint>,
    step: f32,
    tol: f32,
    selected_point_idx: &mut Option<usize>,
    _calibration: &Calibration,
) -> Option<String> {
    let img = match raw_image {
        Some(i) => i.to_rgb8(),
        None => return None,
    };

    let w = img.width() as i32;
    let h = img.height() as i32;
    let step = step.round() as i32;
    let tol = tol as f64;
    let r_limit = tol.round() as i32;

    let manual_count = points
        .iter()
        .filter(|dp| dp.source == PointSource::Manual)
        .count();

    let single_selected_idx = if manual_count == 1 {
        if let Some(idx) = *selected_point_idx {
            if idx < points.len() && points[idx].source == PointSource::Manual {
                Some(idx)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if manual_count < 2 && single_selected_idx.is_none() {
        return Some(
            "Place 2+ manual points or select 1 manual point to enable auto tracing".to_string(),
        );
    }

    let is_light_bg = {
        let mut sum = 0.0;
        let mut count = 0;
        for gx in 1..10 {
            for gy in 1..10 {
                let cx = (w * gx) / 10;
                let cy = (h * gy) / 10;
                if cx < w && cy < h {
                    let pix = img.get_pixel(cx as u32, cy as u32);
                    let brightness =
                        0.299 * pix[0] as f64 + 0.587 * pix[1] as f64 + 0.114 * pix[2] as f64;
                    sum += brightness;
                    count += 1;
                }
            }
        }
        if count > 0 {
            sum / count as f64 > 128.0
        } else {
            true
        }
    };

    let get_color_center = |pt: Point2D| -> Point2D {
        let start_x = pt.x.round() as i32;
        let start_y = pt.y.round() as i32;

        if start_x < 0 || start_x >= w || start_y < 0 || start_y >= h {
            return pt;
        }

        let r_search = r_limit;
        let mut target_y = start_y;

        if is_light_bg {
            let mut min_brightness = f64::MAX;
            for dy in -r_search..=r_search {
                let py = start_y + dy;
                if py >= 0 && py < h {
                    let pix = img.get_pixel(start_x as u32, py as u32);
                    let brightness =
                        0.299 * pix[0] as f64 + 0.587 * pix[1] as f64 + 0.114 * pix[2] as f64;
                    if brightness < min_brightness {
                        min_brightness = brightness;
                        target_y = py;
                    }
                }
            }
        } else {
            let mut max_brightness = -1.0;
            for dy in -r_search..=r_search {
                let py = start_y + dy;
                if py >= 0 && py < h {
                    let pix = img.get_pixel(start_x as u32, py as u32);
                    let brightness =
                        0.299 * pix[0] as f64 + 0.587 * pix[1] as f64 + 0.114 * pix[2] as f64;
                    if brightness > max_brightness {
                        max_brightness = brightness;
                        target_y = py;
                    }
                }
            }
        }

        let target_pixel = img.get_pixel(start_x as u32, target_y as u32);
        let target_r = target_pixel[0] as f64;
        let target_g = target_pixel[1] as f64;
        let target_b = target_pixel[2] as f64;

        let color_dist = |r: u8, g: u8, b: u8| -> f64 {
            let dr = r as f64 - target_r;
            let dg = g as f64 - target_g;
            let db = b as f64 - target_b;
            (dr * dr + dg * dg + db * db).sqrt()
        };

        let mut sum_y = 0.0;
        let mut count = 0;

        for dy in -r_limit..=r_limit {
            let py = target_y + dy;
            if py >= 0 && py < h {
                let pix = img.get_pixel(start_x as u32, py as u32);
                if color_dist(pix[0], pix[1], pix[2]) <= tol {
                    sum_y += py as f64;
                    count += 1;
                }
            }
        }

        if count > 0 {
            Point2D {
                x: pt.x,
                y: sum_y / count as f64,
            }
        } else {
            Point2D {
                x: pt.x,
                y: target_y as f64,
            }
        }
    };

    let selected_manual_pt = selected_point_idx.and_then(|idx| {
        if idx < points.len() && points[idx].source == PointSource::Manual {
            Some(points[idx].point)
        } else {
            None
        }
    });

    // Snap all manual points
    for i in 0..points.len() {
        if points[i].source == PointSource::Manual {
            points[i].point = get_color_center(points[i].point);
        }
    }

    // Clear all previous auto-traced points, keeping only manual points
    points.retain(|dp| dp.source == PointSource::Manual);

    // Sort manual points by X coordinate
    points.sort_by(|a, b| {
        a.point
            .x
            .partial_cmp(&b.point.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Restore selected point index
    if let Some(old_pt) = selected_manual_pt {
        let mut best_idx = None;
        let mut min_d = f64::MAX;
        for (idx, dp) in points.iter().enumerate() {
            if dp.source == PointSource::Manual {
                let d = (dp.point.x - old_pt.x).powi(2) + (dp.point.y - old_pt.y).powi(2);
                if d < min_d {
                    min_d = d;
                    best_idx = Some(idx);
                }
            }
        }
        if min_d < (tol * 2.0).powi(2) {
            *selected_point_idx = best_idx;
        } else {
            *selected_point_idx = None;
        }
    } else {
        *selected_point_idx = None;
    }

    let mut new_points = Vec::new();
    let search_y_range = 60;
    let search_y_range = search_y_range.max(r_limit);

    if manual_count >= 2 {
        for i in 0..(points.len() - 1) {
            let start_dp = points[i];
            let end_dp = points[i + 1];

            let start_x = start_dp.point.x.round() as i32;
            let start_y = start_dp.point.y.round() as i32;

            if start_x < 0 || start_x >= w || start_y < 0 || start_y >= h {
                continue;
            }

            let target_pixel = img.get_pixel(start_x as u32, start_y as u32);
            let target_r = target_pixel[0] as f64;
            let target_g = target_pixel[1] as f64;
            let target_b = target_pixel[2] as f64;

            let color_dist = |r: u8, g: u8, b: u8| -> f64 {
                let dr = r as f64 - target_r;
                let dg = g as f64 - target_g;
                let db = b as f64 - target_b;
                (dr * dr + dg * dg + db * db).sqrt()
            };

            let mut last_valid_y = start_dp.point.y;
            let mut x = start_x + step;
            let limit_x = end_dp.point.x.round() as i32;

            while x < limit_x {
                let search_center_i = last_valid_y.round() as i32;

                let mut best_y = search_center_i;
                let mut min_dist = f64::MAX;

                let y_min = (search_center_i - search_y_range).max(0);
                let y_max = (search_center_i + search_y_range).min(h - 1);

                for y in y_min..=y_max {
                    let px = img.get_pixel(x as u32, y as u32);
                    let dist = color_dist(px[0], px[1], px[2]);
                    if dist < min_dist {
                        min_dist = dist;
                        best_y = y;
                    }
                }

                let mut top_y = best_y;
                while top_y > y_min {
                    let px = img.get_pixel(x as u32, (top_y - 1) as u32);
                    if color_dist(px[0], px[1], px[2]) <= tol {
                        top_y -= 1;
                    } else {
                        break;
                    }
                }

                let mut bot_y = best_y;
                while bot_y < y_max {
                    let px = img.get_pixel(x as u32, (bot_y + 1) as u32);
                    if color_dist(px[0], px[1], px[2]) <= tol {
                        bot_y += 1;
                    } else {
                        break;
                    }
                }

                let center_y = (top_y + bot_y) as f64 / 2.0;

                let expected_y = start_dp.point.y
                    + (end_dp.point.y - start_dp.point.y)
                        * ((x as f64 - start_x as f64) / (limit_x as f64 - start_x as f64));
                if min_dist > 50.0 {
                    last_valid_y = (center_y + expected_y) / 2.0;
                } else {
                    last_valid_y = center_y;
                }

                new_points.push(DataPoint {
                    point: Point2D {
                        x: x as f64,
                        y: center_y,
                    },
                    source: PointSource::AutoTrace,
                });
                x += step;
            }
        }
        let total_added = new_points.len();
        points.extend(new_points);
        Some(format!(
            "Auto-extracted {} points between manual guides!",
            total_added
        ))
    } else if let Some(idx) = single_selected_idx {
        let start_dp = points[idx];
        let start_x = start_dp.point.x.round() as i32;
        let start_y = start_dp.point.y.round() as i32;

        if start_x >= 0 && start_x < w && start_y >= 0 && start_y < h {
            let target_pixel = img.get_pixel(start_x as u32, start_y as u32);
            let target_r = target_pixel[0] as f64;
            let target_g = target_pixel[1] as f64;
            let target_b = target_pixel[2] as f64;

            let color_dist = |r: u8, g: u8, b: u8| -> f64 {
                let dr = r as f64 - target_r;
                let dg = g as f64 - target_g;
                let db = b as f64 - target_b;
                (dr * dr + dg * dg + db * db).sqrt()
            };

            let mut curr_y = start_y;
            let mut x = start_x + step;
            while x < w {
                let mut best_y = curr_y;
                let mut min_dist = f64::MAX;

                let y_min = (curr_y - search_y_range).max(0);
                let y_max = (curr_y + search_y_range).min(h - 1);

                for y in y_min..=y_max {
                    let px = img.get_pixel(x as u32, y as u32);
                    let dist = color_dist(px[0], px[1], px[2]);
                    if dist < min_dist {
                        min_dist = dist;
                        best_y = y;
                    }
                }

                if min_dist <= tol {
                    let mut top_y = best_y;
                    while top_y > y_min {
                        let px = img.get_pixel(x as u32, (top_y - 1) as u32);
                        if color_dist(px[0], px[1], px[2]) <= tol {
                            top_y -= 1;
                        } else {
                            break;
                        }
                    }

                    let mut bot_y = best_y;
                    while bot_y < y_max {
                        let px = img.get_pixel(x as u32, (bot_y + 1) as u32);
                        if color_dist(px[0], px[1], px[2]) <= tol {
                            bot_y += 1;
                        } else {
                            break;
                        }
                    }

                    let center_y = (top_y + bot_y) as f64 / 2.0;
                    new_points.push(DataPoint {
                        point: Point2D {
                            x: x as f64,
                            y: center_y,
                        },
                        source: PointSource::AutoTrace,
                    });
                    curr_y = center_y.round() as i32;
                    x += step;
                } else {
                    break;
                }
            }
        }
        let total_added = new_points.len();
        points.extend(new_points);
        Some(format!(
            "Auto-extracted {} points to the right starting from color center!",
            total_added
        ))
    } else {
        None
    }
}
