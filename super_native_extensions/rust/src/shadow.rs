use crate::{
    api_model::{ImageData, TargettedImage},
    blur::blur_image_data,
};

fn inflate_image_data(source: &ImageData, padding: i32) -> ImageData {
    let new_width = source.width + 2 * padding;
    let new_height = source.height + 2 * padding;
    let mut res = ImageData {
        width: new_width,
        height: new_height,
        bytes_per_row: new_width * 4,
        data: vec![0; (new_width * new_height * 4) as usize],
        device_pixel_ratio: source.device_pixel_ratio,
    };

    let line_length = (source.width * 4) as usize;
    for y in 0..source.height {
        let dest_start = ((y + padding) * res.bytes_per_row + padding * 4) as usize;
        let src_start = (y * source.bytes_per_row) as usize;
        res.data[dest_start..dest_start + line_length]
            .copy_from_slice(&source.data[src_start..src_start + line_length]);
    }
    res
}

fn draw_shadow(image: &mut ImageData, radius: i32) {
    assert!(image.bytes_per_row == image.width * 4);

    let data = &mut image.data;

    let mut shadow = vec![0u8; (image.width * image.height) as usize];

    (0..data.len() / 4).for_each(|i| {
        shadow[i] = data[i * 4 + 3] / 2;
    });

    blur_image_data(
        &mut shadow,
        0,
        0,
        image.width as usize,
        image.height as usize,
        radius as usize,
    );

    (0..data.len() / 4).for_each(|i| {
        let index = i * 4;
        let a0_ = data[index + 3];
        if a0_ == 255 {
            // full opacity, no shadow
        } else if a0_ == 0 {
            // zero opacity, only shadow
            data[index] = 0;
            data[index + 1] = 0;
            data[index + 2] = 0;
            data[index + 3] = shadow[i];
        } else {
            // blend
            let r0 = f64::from(data[index]) / 255.0;
            let g0 = f64::from(data[index + 1]) / 255.0;
            let b0 = f64::from(data[index + 2]) / 255.0;
            let a0 = f64::from(a0_) / 255.0;
            let a1 = f64::from(shadow[i]) / 255.0;

            let a = a0 + a1 * (1.0 - a0);
            let r = (r0 * a0) / a;
            let g = (g0 * a0) / a;
            let b = (b0 * a0) / a;

            data[index] = (r * 255.0) as u8;
            data[index + 1] = (g * 255.0) as u8;
            data[index + 2] = (b * 255.0) as u8;
            data[index + 3] = (a * 255.0) as u8;
        }
    });
}

pub trait WithShadow {
    fn with_shadow(&self, radius: i32) -> Self;
}

impl WithShadow for TargettedImage {
    fn with_shadow(&self, radius: i32) -> Self {
        let adjusted_radius =
            ((radius as f64) * self.image_data.device_pixel_ratio.unwrap_or(1.0)) as i32;
        let mut image_data = inflate_image_data(&self.image_data, adjusted_radius);
        draw_shadow(&mut image_data, adjusted_radius);
        TargettedImage {
            image_data,
            rect: self.rect.inflated(radius as f64, radius as f64),
        }
    }
}
