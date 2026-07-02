use crate::calibration::{Calibration, Point2D};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DragTarget {
    X1,
    X2,
    Y1,
    Y2,
    DataPoint(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PickTarget {
    X1,
    X2,
    Y1,
    Y2,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SortOrder {
    None,
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PointSource {
    Manual,
    AutoTrace,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RightTab {
    Points,
    Plot,
    KuvaPlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DataPoint {
    pub point: Point2D,
    pub source: PointSource,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DataPointDerive {
    WithSource { point: Point2D, source: PointSource },
    JustPoint(Point2D),
}

impl From<DataPointDerive> for DataPoint {
    fn from(d: DataPointDerive) -> Self {
        match d {
            DataPointDerive::WithSource { point, source } => DataPoint { point, source },
            DataPointDerive::JustPoint(point) => DataPoint {
                point,
                source: PointSource::Manual,
            },
        }
    }
}

impl<'de> Deserialize<'de> for DataPoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        DataPointDerive::deserialize(deserializer).map(Self::from)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProjectData {
    pub image_path: Option<String>,
    pub calibration: Calibration,
    pub points: Vec<DataPoint>,
    pub x_label: String,
    pub y_label: String,
    pub zoom_factor: Option<f32>,
    pub pan_offset: Option<[f32; 2]>,
    pub sort_order: Option<SortOrder>,
    pub include_errors: Option<bool>,
    pub connect_lines: Option<bool>,
    pub marker_size: Option<f32>,
    pub auto_trace_step: Option<f32>,
    pub auto_trace_tol: Option<f32>,
    pub right_tab: Option<RightTab>,
    pub x1_input: Option<String>,
    pub x2_input: Option<String>,
    pub y1_input: Option<String>,
    pub y2_input: Option<String>,
}
