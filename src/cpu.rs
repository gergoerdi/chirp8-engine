pub use prelude::*;
pub use opcodes::*;
pub use peripherals::Peripherals;

use core::marker::PhantomData;

pub trait Quirks {
    const SHIFT_VY: bool;
    const RESET_VF: bool;
    const INCREMENT_PTR: bool;
    const VIDEO_WAIT: bool;
    const CLIP_SPRITES: bool;
}

pub struct DefaultQuirks;

impl Quirks for DefaultQuirks {
    const SHIFT_VY: bool = true;
    const RESET_VF: bool = true;
    const INCREMENT_PTR: bool = true;
    const VIDEO_WAIT: bool = true;
    const CLIP_SPRITES: bool = true;
}

enum State {
    Running,
    WaitPress(Byte, u16),
    WaitRelease(Byte),
    WaitFrame,
}

pub struct CPU<Q: Quirks> {
    quirks: PhantomData<Q>,
    regs: [Byte; 16],
    ptr: Addr,
    pc: Addr,
    stack: [Addr; 16],
    sp: usize,
    rnd: Addr,
    timer: Byte,
    state: State,
}

impl<Q: Quirks> CPU<Q> {
    pub const fn new() -> Self {
        CPU{
            quirks: PhantomData,
            regs : [0; 16],
            ptr: 0,
            pc: 0x200,
            stack: [0; 16],
            sp: 0,
            rnd: 0xf00f,
            timer: 0,
            state: State::Running,
        }
    }

    pub fn tick_frame(&mut self) {
        if self.timer > 0 { self.timer -= 1 };
        self.next_random();

        if let State::WaitFrame = self.state {
            self.state = State::Running;
        }
    }

    fn eval(&self, arg: Arg) -> Byte {
        match arg {
            Arg::Reg(vx) => self.regs[vx as usize],
            Arg::Imm(nn) => nn
        }
    }

    fn arith(op: Arith, x: Byte, y: Byte) -> (Byte, Option<bool>) {
        match op {
            Arith::Load => (y, None),
            Arith::Or => (x | y, if Q::RESET_VF { Some(false) } else { None }),
            Arith::And => (x & y, if Q::RESET_VF { Some(false) } else { None }),
            Arith::XOr => (x ^ y, if Q::RESET_VF { Some(false) } else { None }),
            Arith::Add => {
                let (z, f) = u8::overflowing_add(x, y);
                (z, Some(f))
            },
            Arith::Sub => {
                let (z, f) = u8::overflowing_sub(x, y);
                (z, Some(!f))
            },
            Arith::SubFlip => {
                let (z, f) = u8::overflowing_sub(y, x);
                (z, Some(!f))
            },
            Arith::ShiftL => {
                let arg = if Q::SHIFT_VY { y } else { x };
                (arg << 1, Some(arg & 0x80 != 0))
            },
            Arith::ShiftR => {
                let arg = if Q::SHIFT_VY { y } else { x };
                (arg >> 1, Some(arg & 0x01 != 0))
            }
        }
    }

    fn set_flag(&mut self, flag: bool) {
        self.regs[0xf] = if flag { 1 } else { 0 };
    }

    /// 16-bit LFSR a la https://en.wikipedia.org/wiki/Linear-feedback_shift_register
    fn next_random(&mut self) -> Byte {
        let lsb = self.rnd & 1 != 0;
        self.rnd >>= 1;
        if lsb { self.rnd ^= 0xb400 }
        self.rnd as Byte
    }

    pub fn step<P>(&mut self, io: &mut P) where P: Peripherals {
        match self.state {
            State::Running => self.exec(io),
            State::WaitFrame => (),
            State::WaitPress(vx, prev_key_state) => {
                let new_state = io.get_keys();

                let fresh_keys = new_state & !prev_key_state;
                let idx = fresh_keys.trailing_zeros() as Byte;
                if idx < 16 {
                    self.regs[vx as usize] = idx;
                    self.state = State::WaitRelease(idx);
                } else {
                    self.state = State::WaitPress(vx, new_state);
                }
            },
            State::WaitRelease(key) => {
                let pressed = io.get_keys() & (1 << key) != 0;
                if !pressed {
                    self.state = State::Running;
                }
            }
        }
    }

    pub fn exec<P>(&mut self, io: &mut P) where P: Peripherals {
        let hi = io.read_ram(self.pc); self.pc += 1;
        let lo = io.read_ram(self.pc); self.pc += 1;

        // match decode(hi, lo) {
        //     None => println!("0x{:04x} 0x{:02x} 0x{:02x}", self.pc-2, hi, lo),
        //     Some(op) => println!("0x{:04x} {:?}", self.pc-2, op)
        // }

        match decode(hi, lo).unwrap() {
            Op::Sys(_addr) => {
                // TODO
            },
            Op::Call(addr) => {
                self.stack[self.sp] = self.pc;
                self.sp = (self.sp + 1) & 0x0f;
                self.pc = addr;
            },
            Op::Ret => {
                self.sp = (self.sp - 1) & 0x0f;
                self.pc = self.stack[self.sp];
            },
            Op::Jump(addr) => {
                self.pc = addr;
            },
            Op::Skip(when, vx, target) => {
                let x = self.regs[vx as usize];
                let y = self.eval(target);
                let skip = match when {
                    Cmp::Eq => x == y,
                    Cmp::NEq => x != y
                };
                if skip {
                    self.pc += 2;
                }
            },
            Op::LoadImm(vx, imm) => {
                self.regs[vx as usize] = imm
            },
            Op::AddImm(vx, imm) => {
                self.regs[vx as usize] = u8::wrapping_add(self.regs[vx as usize], imm)
            },
            Op::Arith(op, vx, vy) => {
                let x = self.regs[vx as usize];
                let y = self.regs[vy as usize];
                let (z, flag) = Self::arith(op, x, y);
                self.regs[vx as usize] = z;
                flag.map(|flag| { self.set_flag(flag); });
            },
            Op::LoadI(addr) => {
                self.ptr = addr;
            },
            Op::AddI(vx) => {
                let addr = self.ptr + self.regs[vx as usize] as u16;
                self.set_flag(addr > 0x0fff);
                self.ptr = addr & 0x0fff;
            },
            Op::GetTimer(vx) => {
               self.regs[vx as usize] = self.timer;
            },
            Op::SetTimer(vx) => {
                self.timer = self.regs[vx as usize];
            },
            Op::JumpV0(addr) => {
                self.pc = addr + self.regs[0] as Addr;
            },
            Op::Random(vx, mask) => {
                self.regs[vx as usize] = self.next_random() & mask;
            },
            Op::Hex(vx) => {
                self.ptr = (self.regs[vx as usize] as u16 & 0x0f) << 3;
            },
            Op::StoreBCD(vx) => {
                let x = self.regs[vx as usize];
                io.write_ram(self.ptr, x / 100);
                io.write_ram(self.ptr + 1, (x % 100) / 10);
                io.write_ram(self.ptr + 2, x % 10);
                if Q::INCREMENT_PTR {
                    self.ptr += 3;
                }
            },
            Op::Save(vx) => {
                for i in 0..vx as usize +1 {
                    io.write_ram(self.ptr + i as Addr, self.regs[i])
                }
                if Q::INCREMENT_PTR {
                    self.ptr += 3;
                }
            },
            Op::Restore(vx) => {
                for i in 0..vx as usize +1 {
                    self.regs[i] = io.read_ram(self.ptr + i as Addr)
                }
            },
            Op::Draw(vx, vy, n) => {
                let mut collision = false;

                let yd0 = self.regs[vy as usize] & 0x1f;
                let xd = self.regs[vx as usize] & 0x3f;

                for i in 0..n {
                    let yd = yd0 + i;
                    if Q::CLIP_SPRITES && yd > 31 { break }

                    let yd = yd & 0x1f;
                    let dat = io.read_ram(self.ptr + i as Addr);
                    let row0 = (dat as ScreenRow) << 56;
                    let row = if Q::CLIP_SPRITES { row0 >> xd } else { row0.rotate_right(xd as u32) };

                    let old_row = io.get_pixel_row(yd);
                    let new_row = old_row ^ row;
                    collision |= old_row & row != 0;
                    io.set_pixel_row(yd, new_row);
                };
                self.set_flag(collision);
                if Q::VIDEO_WAIT { self.state = State::WaitFrame };
            },
            Op::ClearScr => {
                for y in 0..32 {
                    io.set_pixel_row(y, 0);
                }
                if Q::VIDEO_WAIT { self.state = State::WaitFrame };
            },
            Op::SkipKey(cond, vx) => {
                let pressed = io.get_keys() & (1 << self.regs[vx as usize]) != 0;
                let target = match cond {
                    Cmp::Eq => true,
                    Cmp::NEq => false
                };
                if pressed == target {
                    self.pc += 2;
                }
            },
            Op::WaitKey(vx) => {
                self.state = State::WaitPress(vx, 0xffff);
            },
            Op::SetSound(vx) => {
                io.set_sound(self.regs[vx as usize]);
            },
        }
    }
}
