use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl BBox {
    pub fn width(self) -> f32 {
        (self.x1 - self.x0).max(0.0)
    }

    pub fn height(self) -> f32 {
        (self.y1 - self.y0).max(0.0)
    }

    pub fn area(self) -> f32 {
        bbox_area(&self)
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.x0, self.y0, self.x1, self.y1]
    }

    pub fn from_array(arr: [f32; 4]) -> Self {
        Self {
            x0: arr[0],
            y0: arr[1],
            x1: arr[2],
            y1: arr[3],
        }
    }
}

pub fn bbox_area(b: &BBox) -> f32 {
    b.width() * b.height()
}

pub fn bbox_intersection(a: &BBox, b: &BBox) -> f32 {
    let x0 = a.x0.max(b.x0);
    let y0 = a.y0.max(b.y0);
    let x1 = a.x1.min(b.x1);
    let y1 = a.y1.min(b.y1);

    if x1 <= x0 || y1 <= y0 {
        0.0
    } else {
        (x1 - x0) * (y1 - y0)
    }
}

pub fn bbox_iou(a: &BBox, b: &BBox) -> f32 {
    let intersection = bbox_intersection(a, b);
    if intersection <= 0.0 {
        return 0.0;
    }

    let union = bbox_area(a) + bbox_area(b) - intersection;
    if union <= 0.0 {
        0.0
    } else {
        (intersection / union).clamp(0.0, 1.0)
    }
}
