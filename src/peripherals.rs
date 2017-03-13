use prelude::*;

pub const SCREEN_WIDTH : u8 = 84;
pub const SCREEN_HEIGHT : u8 = 48;

pub trait Peripherals {
    fn keep_running(&self) -> bool;

    fn clear_pixels(&self);
    fn set_pixel(&self, Byte, Byte, bool);
    fn get_pixel(&self, Byte, Byte) -> bool;
    fn redraw(&self);

    fn scan_key_row(&self, Byte) -> Byte;

    fn set_timer(&self, Byte);
    fn get_timer(&self) -> Byte;
    fn set_sound(&self, Byte);

    fn read_ram(&self, Addr) -> Byte;
    fn write_ram(&self, Addr, Byte);

    fn get_random(&self) -> Byte;
}
