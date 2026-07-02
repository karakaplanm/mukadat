use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CalibrationPoint {
    pub pixel: Option<Point2D>,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calibration {
    pub x1: CalibrationPoint,
    pub x2: CalibrationPoint,
    pub y1: CalibrationPoint,
    pub y2: CalibrationPoint,
    pub x_log: bool,
    pub y_log: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CalibratedValue {
    pub x_val: f64,
    pub y_val: f64,
    pub x_err: f64,
    pub y_err: f64,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            x1: CalibrationPoint {
                pixel: None,
                value: None,
            },
            x2: CalibrationPoint {
                pixel: None,
                value: None,
            },
            y1: CalibrationPoint {
                pixel: None,
                value: None,
            },
            y2: CalibrationPoint {
                pixel: None,
                value: None,
            },
            x_log: false,
            y_log: false,
        }
    }
}

impl Calibration {
    pub fn is_fully_configured(&self) -> bool {
        self.x1.pixel.is_some()
            && self.x1.value.is_some()
            && self.x2.pixel.is_some()
            && self.x2.value.is_some()
            && self.y1.pixel.is_some()
            && self.y1.value.is_some()
            && self.y2.pixel.is_some()
            && self.y2.value.is_some()
    }

    pub fn calculate(&self, pixel: Point2D) -> Option<CalibratedValue> {
        if !self.is_fully_configured() {
            return None;
        }

        let p_x1 = self.x1.pixel.unwrap();
        let p_x2 = self.x2.pixel.unwrap();
        let p_y1 = self.y1.pixel.unwrap();
        let p_y2 = self.y2.pixel.unwrap();

        let v_x1 = self.x1.value.unwrap();
        let v_x2 = self.x2.value.unwrap();
        let v_y1 = self.y1.value.unwrap();
        let v_y2 = self.y2.value.unwrap();

        // If log scale, we must check values > 0
        if self.x_log && (v_x1 <= 0.0 || v_x2 <= 0.0) {
            return None;
        }
        if self.y_log && (v_y1 <= 0.0 || v_y2 <= 0.0) {
            return None;
        }

        // Apply log transform to real values if needed
        let r_x1 = if self.x_log { v_x1.ln() } else { v_x1 };
        let r_x2 = if self.x_log { v_x2.ln() } else { v_x2 };
        let r_y1 = if self.y_log { v_y1.ln() } else { v_y1 };
        let r_y2 = if self.y_log { v_y2.ln() } else { v_y2 };

        // Vectors in pixel coordinates
        let x21 = p_x2.x - p_x1.x;
        let y21 = p_x2.y - p_x1.y;
        let x43 = p_y2.x - p_y1.x;
        let y43 = p_y2.y - p_y1.y;

        // Prevent division by zero
        if y43 == 0.0 || x21 == 0.0 {
            return None;
        }

        let denom_alpha = x21 - (y21 * x43) / y43;
        let denom_beta = y43 - (x43 * y21) / x21;
        if denom_alpha == 0.0 || denom_beta == 0.0 {
            return None;
        }

        let calc_alpha_beta = |p: Point2D| -> (f64, f64) {
            let alpha = ((p_x1.x - p.x) - (p_x1.y - p.y) * (x43 / y43)) / denom_alpha;
            let beta = ((p_y1.y - p.y) - (p_y1.x - p.x) * (y21 / x21)) / denom_beta;
            (alpha, beta)
        };

        let map_val = |alpha: f64, beta: f64| -> (f64, f64) {
            let x_val_transformed = -alpha * (r_x2 - r_x1) + r_x1;
            let y_val_transformed = -beta * (r_y2 - r_y1) + r_y1;

            let x_val = if self.x_log {
                x_val_transformed.exp()
            } else {
                x_val_transformed
            };
            let y_val = if self.y_log {
                y_val_transformed.exp()
            } else {
                y_val_transformed
            };
            (x_val, y_val)
        };

        // Current point value
        let (alpha, beta) = calc_alpha_beta(pixel);
        let (x_val, y_val) = map_val(alpha, beta);

        // Values at +1, +1 pixel offset
        let (alpha_p1, beta_p1) = calc_alpha_beta(Point2D {
            x: pixel.x + 1.0,
            y: pixel.y + 1.0,
        });
        let (x_val_p1, y_val_p1) = map_val(alpha_p1, beta_p1);

        // Values at -1, -1 pixel offset
        let (alpha_m1, beta_m1) = calc_alpha_beta(Point2D {
            x: pixel.x - 1.0,
            y: pixel.y - 1.0,
        });
        let (x_val_m1, y_val_m1) = map_val(alpha_m1, beta_m1);

        // Errors: |V(+1) - V(-1)| / 4
        let x_err = (x_val_p1 - x_val_m1).abs() / 4.0;
        let y_err = (y_val_p1 - y_val_m1).abs() / 4.0;

        Some(CalibratedValue {
            x_val,
            y_val,
            x_err,
            y_err,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_calibration() {
        let calibration = Calibration {
            x1: CalibrationPoint {
                pixel: Some(Point2D { x: 100.0, y: 500.0 }),
                value: Some(0.0),
            },
            x2: CalibrationPoint {
                pixel: Some(Point2D { x: 500.0, y: 500.0 }),
                value: Some(10.0),
            },
            y1: CalibrationPoint {
                pixel: Some(Point2D { x: 100.0, y: 500.0 }),
                value: Some(0.0),
            },
            y2: CalibrationPoint {
                pixel: Some(Point2D { x: 100.0, y: 100.0 }),
                value: Some(5.0),
            },
            x_log: false,
            y_log: false,
        };

        // Center point: (300, 300) should be x=5.0, y=2.5
        let res = calibration
            .calculate(Point2D { x: 300.0, y: 300.0 })
            .unwrap();
        assert!((res.x_val - 5.0).abs() < 1e-5);
        assert!((res.y_val - 2.5).abs() < 1e-5);

        // (500, 100) should be x=10.0, y=5.0
        let res2 = calibration
            .calculate(Point2D { x: 500.0, y: 100.0 })
            .unwrap();
        assert!((res2.x_val - 10.0).abs() < 1e-5);
        assert!((res2.y_val - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_logarithmic_calibration() {
        let calibration = Calibration {
            x1: CalibrationPoint {
                pixel: Some(Point2D { x: 100.0, y: 500.0 }),
                value: Some(1.0),
            },
            x2: CalibrationPoint {
                pixel: Some(Point2D { x: 500.0, y: 500.0 }),
                value: Some(100.0),
            },
            y1: CalibrationPoint {
                pixel: Some(Point2D { x: 100.0, y: 500.0 }),
                value: Some(10.0),
            },
            y2: CalibrationPoint {
                pixel: Some(Point2D { x: 100.0, y: 100.0 }),
                value: Some(1000.0),
            },
            x_log: true,
            y_log: true,
        };

        // Center point: (300, 300) should be x=sqrt(1*100)=10, y=sqrt(10*1000)=100
        let res = calibration
            .calculate(Point2D { x: 300.0, y: 300.0 })
            .unwrap();
        assert!((res.x_val - 10.0).abs() < 1e-5);
        assert!((res.y_val - 100.0).abs() < 1e-5);
    }
}
