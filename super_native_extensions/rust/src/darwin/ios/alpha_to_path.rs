use objc2::rc::Id;
use objc2_foundation::{CGPoint, CGRect, CGSize};
use objc2_ui_kit::UIBezierPath;

use crate::api_model::ImageData;

struct AlphaUtil<'a> {
    image_data: &'a ImageData,
}

struct Span {
    start: i32,
    end: i32,
}

#[derive(Copy, Clone, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl<'a> AlphaUtil<'a> {
    fn is_opaque(&self, x: i32, y: i32) -> bool {
        let alpha = self.image_data.data[(y * self.image_data.bytes_per_row + x * 4 + 3) as usize];
        alpha > 128
    }

    fn spans_for_line(&self, y: i32, spans: &mut Vec<Span>) {
        let mut start = -1;
        let mut end = -1;
        spans.clear();
        for x in 0..self.image_data.width {
            if self.is_opaque(x, y) {
                if start == -1 {
                    start = x;
                }
                end = x;
            } else if start != -1 {
                spans.push(Span { start, end });
                start = -1;
                end = -1;
            }
        }
        if start != -1 {
            spans.push(Span { start, end });
        }
    }

    fn rects_for_alpha(&self) -> Vec<Rect> {
        let mut res = Vec::<Rect>::new();
        let mut spans = Vec::<Span>::new();
        let mut active_rects = Vec::<Rect>::new();
        for y in 0..self.image_data.height {
            self.spans_for_line(y, &mut spans);
            spans.retain(|span| {
                for rect in active_rects.iter_mut() {
                    if rect.x1 == span.start && rect.x2 == span.end {
                        rect.y2 = y;
                        return false;
                    }
                }
                true
            });
            active_rects.retain(|rect| {
                if rect.y2 != y {
                    res.push(*rect);
                    false
                } else {
                    true
                }
            });
            for span in spans.iter() {
                active_rects.push(Rect {
                    x1: span.start,
                    y1: y,
                    x2: span.end,
                    y2: y,
                });
            }
        }
        res.append(&mut active_rects);
        res
    }
}

pub fn bezier_path_for_alpha(image_data: &ImageData) -> Id<UIBezierPath> {
    let util = AlphaUtil { image_data };
    let rects = util.rects_for_alpha();
    let path = unsafe { UIBezierPath::bezierPath() };
    for rect in rects.iter() {
        let ratio = image_data.device_pixel_ratio.unwrap_or(1.0);
        let rect = CGRect::new(
            CGPoint::new(rect.x1 as f64 / ratio, rect.y1 as f64 / ratio),
            CGSize::new(
                (rect.x2 - rect.x1) as f64 / ratio,
                (rect.y2 - rect.y1) as f64 / ratio,
            ),
        );
        unsafe {
            let sub_path = UIBezierPath::bezierPathWithRect(rect);
            path.appendPath(&sub_path);
        }
    }
    path
}
