// Original source:
// https://github.com/flozz/StackBlur/blob/master/src/stackblur.js

/*
 * @copyright (c) 2010 Mario Klingemann
 *
 * Permission is hereby granted, free of charge, to any person
 * obtaining a copy of this software and associated documentation
 * files (the "Software"), to deal in the Software without
 * restriction, including without limitation the rights to use,
 * copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following
 * conditions:
 *
 * The above copyright notice and this permission notice shall be
 * included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES
 * OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
 * HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
 * WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
 * OTHER DEALINGS IN THE SOFTWARE.
 */

use std::cmp::min;

pub fn blur_image_data(
    pixels: &mut [u8],
    top_x: usize,
    top_y: usize,
    width: usize,
    height: usize,
    radius: usize,
) {
    let div = 2 * radius + 1;
    let width_minus_1 = width - 1;
    let height_minus_1 = height - 1;
    let radius_plus_1 = radius + 1;
    let sum_factor = radius_plus_1 * (radius_plus_1 + 1) / 2;

    let mut stack = vec![0_usize; div];

    let stack_start: usize = 0;
    let stack_end = radius_plus_1;

    let mut stack_in;
    let mut stack_out;

    let mul_sum = MUL_TABLE[radius] as usize;
    let shg_sum = SHG_TABLE[radius] as usize;

    let mut p: usize;
    let mut yw = 0;
    let mut yi = top_y * width + top_x;

    for _y in 0..height {
        let mut pr = pixels[yi] as usize;
        let mut r_out_sum = radius_plus_1 * pr;
        let mut r_sum = sum_factor * pr;

        (0..radius_plus_1).for_each(|i| {
            stack[i] = pr;
        });

        let mut r_in_sum = 0;
        for i in 1..radius_plus_1 {
            p = yi + min(width_minus_1, i);
            let val = pixels[p] as usize;
            stack[i - 1 + radius_plus_1] = val;
            r_sum += val * (radius_plus_1 - i);
            r_in_sum += val;
        }

        stack_in = stack_start;
        stack_out = stack_end;
        for x in 0..width {
            pixels[yi] = ((r_sum * mul_sum) >> shg_sum) as u8;
            r_sum -= r_out_sum;
            r_out_sum -= stack[stack_in];
            p = x + radius + 1;
            p = yw + min(width_minus_1, p);

            let val = pixels[p] as usize;
            stack[stack_in] = val;
            r_in_sum += val;
            r_sum += r_in_sum;
            stack_in = (stack_in + 1) % div;
            pr = stack[stack_out];
            r_out_sum += pr;
            r_in_sum -= pr;
            stack_out = (stack_out + 1) % div;
            yi += 1;
        }
        yw += width;
    }

    for x in 0..width {
        let mut yi = top_y * width + top_x + x;
        let mut pr = pixels[yi] as usize;
        let mut r_out_sum = radius_plus_1 * pr;
        let mut r_sum = sum_factor * pr;

        (0..radius_plus_1).for_each(|i| {
            stack[i] = pr;
        });

        let mut r_in_sum = 0;
        let mut yp = width;
        for i in 1..=radius {
            yi = yp + x;
            let pr = pixels[yi] as usize;
            stack[i - 1 + radius_plus_1] = pr;
            r_sum += pr * (radius_plus_1 - i);
            r_in_sum += pr;

            if i < height_minus_1 {
                yp += width;
            }
        }

        yi = x;
        stack_in = stack_start;
        stack_out = stack_end;

        for y in 0..height {
            p = yi;
            pixels[p] = ((r_sum * mul_sum) >> shg_sum) as u8;
            r_sum -= r_out_sum;
            r_out_sum -= stack[stack_in];
            p = y + radius_plus_1;
            p = x + min(height_minus_1, p) * width;
            pr = pixels[p] as usize;
            stack[stack_in] = pr;
            r_in_sum += pr;
            r_sum += r_in_sum;
            stack_in = (stack_in + 1) % div;
            pr = stack[stack_out];
            r_out_sum += pr;
            r_in_sum -= pr;
            stack_out = (stack_out + 1) % div;
            yi += width;
        }
    }
}

const MUL_TABLE: [i32; 255] = [
    512, 512, 456, 512, 328, 456, 335, 512, 405, 328, 271, 456, 388, 335, 292, 512, 454, 405, 364,
    328, 298, 271, 496, 456, 420, 388, 360, 335, 312, 292, 273, 512, 482, 454, 428, 405, 383, 364,
    345, 328, 312, 298, 284, 271, 259, 496, 475, 456, 437, 420, 404, 388, 374, 360, 347, 335, 323,
    312, 302, 292, 282, 273, 265, 512, 497, 482, 468, 454, 441, 428, 417, 405, 394, 383, 373, 364,
    354, 345, 337, 328, 320, 312, 305, 298, 291, 284, 278, 271, 265, 259, 507, 496, 485, 475, 465,
    456, 446, 437, 428, 420, 412, 404, 396, 388, 381, 374, 367, 360, 354, 347, 341, 335, 329, 323,
    318, 312, 307, 302, 297, 292, 287, 282, 278, 273, 269, 265, 261, 512, 505, 497, 489, 482, 475,
    468, 461, 454, 447, 441, 435, 428, 422, 417, 411, 405, 399, 394, 389, 383, 378, 373, 368, 364,
    359, 354, 350, 345, 341, 337, 332, 328, 324, 320, 316, 312, 309, 305, 301, 298, 294, 291, 287,
    284, 281, 278, 274, 271, 268, 265, 262, 259, 257, 507, 501, 496, 491, 485, 480, 475, 470, 465,
    460, 456, 451, 446, 442, 437, 433, 428, 424, 420, 416, 412, 408, 404, 400, 396, 392, 388, 385,
    381, 377, 374, 370, 367, 363, 360, 357, 354, 350, 347, 344, 341, 338, 335, 332, 329, 326, 323,
    320, 318, 315, 312, 310, 307, 304, 302, 299, 297, 294, 292, 289, 287, 285, 282, 280, 278, 275,
    273, 271, 269, 267, 265, 263, 261, 259,
];

const SHG_TABLE: [i32; 255] = [
    9, 11, 12, 13, 13, 14, 14, 15, 15, 15, 15, 16, 16, 16, 16, 17, 17, 17, 17, 17, 17, 17, 18, 18,
    18, 18, 18, 18, 18, 18, 18, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 20, 20, 20,
    20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 21, 21, 21, 21, 21, 21, 21, 21, 21,
    21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 22, 22, 22, 22, 22, 22,
    22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22,
    22, 22, 22, 22, 22, 22, 22, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
    23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
    23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
];
