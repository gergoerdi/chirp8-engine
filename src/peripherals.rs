use prelude::*;

pub const SCREEN_WIDTH : u8 = 84;
pub const SCREEN_HEIGHT : u8 = 48;

pub trait Peripherals {
    fn keep_running(&self) -> bool;

    fn clear_pixels(&self);
    fn set_pixel(&self, x: Byte, y: Byte, _: bool);
    fn get_pixel(&self, x: Byte, y: Byte) -> bool;
    fn redraw(&self);

    fn scan_key_row(&self, row: Byte) -> Byte;

    fn set_timer(&self, val: Byte);
    fn get_timer(&self) -> Byte;
    fn set_sound(&self, val: Byte);

    fn read_ram(&self, addr: Addr) -> Byte;
    fn write_ram(&self, addr: Addr, val: Byte);

    fn get_random(&self) -> Byte;
}
