use core_graphics::geometry::CGPoint;

use crate::api_model::Point;

impl From<CGPoint> for Point {
    fn from(p: CGPoint) -> Self {
        Point { x: p.x, y: p.y }
    }
}

