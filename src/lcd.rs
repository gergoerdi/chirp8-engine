use prelude::*;
use padded::pad;

pub type FrameBuf = [ScreenRow; SCREEN_HEIGHT as usize];

const COLOR_ON       : u32 = 0xff_00_00_00;
const COLOR_ON_GRID  : u32 = 0xff_20_38_20;
const COLOR_OFF      : u32 = 0xff_73_bd_71;
const COLOR_OFF_GRID : u32 = 0xff_63_ad_61;

pub fn draw_lcd(framebuf: &FrameBuf, pixbuf: &mut [u32], padding: (usize, usize), scaling: (usize, usize)) {
    let (pad_x, pad_y) = padding;
    let (scale_x, scale_y) = scaling;

    let rowstride = scale_x as usize * (SCREEN_WIDTH as usize + 2 * pad_x);

    for (y, yp) in pad(pad_y as usize, 0..SCREEN_HEIGHT as usize).enumerate() {
        let mut row = if let Some(row_idx) = yp { framebuf[row_idx] } else { 0 };

        for (x, xp) in pad(pad_x, 0..SCREEN_WIDTH as usize).enumerate() {
            let pixel = if let Some(_) = xp {
                let pixel = row & (1 << 63) != 0;
                row <<= 1;
                pixel
            } else {
                false
            };

            for i in 0..scale_y {
                for j in 0.. scale_x {
                    let grid_y = (i == 0) | (i == scale_y - 1);
                    let grid_x = (j == 0) | (j == scale_x - 1);

                    let ptr = (y * scale_y + i) * rowstride + (x * scale_x + j);
                    pixbuf[ptr] =
                        if grid_x || grid_y {
                            if pixel { COLOR_ON_GRID } else { COLOR_OFF_GRID }
                        } else {
                            if pixel { COLOR_ON } else { COLOR_OFF }
                        }
                }
            }
        }
    }
}
