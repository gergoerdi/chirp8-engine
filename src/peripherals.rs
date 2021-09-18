use prelude::*;

pub const SCREEN_WIDTH : u8 = 64;
pub const SCREEN_HEIGHT : u8 = 32;

pub trait Peripherals {
    fn keep_running(&self) -> bool;

    fn clear_pixels(&mut self);
    fn set_pixel(&mut self, x: Byte, y: Byte, _: bool);
    fn get_pixel(&self, x: Byte, y: Byte) -> bool;
    fn redraw(&mut self);

    fn get_keys(&self) -> u16;

    fn set_timer(&mut self, val: Byte);
    fn get_timer(&self) -> Byte;
    fn set_sound(&mut self, val: Byte);

    fn read_ram(&self, addr: Addr) -> Byte;
    fn write_ram(&mut self, addr: Addr, val: Byte);

    fn get_random(&mut self) -> Byte;
}
