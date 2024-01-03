pub use prelude::*;
pub use opcodes::*;
pub use peripherals::Peripherals;

pub struct CPU {
    regs: [Byte; 16],
    addr: Addr,
    pc: Addr,
    stack: [Addr; 16],
    sp: usize,
    rnd: Addr,
    timer: Byte,
    prev_key_state: u16,
    prev_key: Option<u8>,
}

impl CPU {
    pub const fn new() -> CPU {
        CPU{ regs : [0; 16],
             addr: 0,
             pc: 0x200,
             stack: [0; 16],
             sp: 0,
             rnd: 0xf00f,
             timer: 0,
             prev_key_state: 0xffff,
             prev_key: None,
        }
    }

    pub fn tick_frame(&mut self) {
        if self.timer > 0 { self.timer -= 1 };
        self.next_random();
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
            Arith::Or => (x | y, Some(false)),
            Arith::And => (x & y, Some(false)),
            Arith::XOr => (x ^ y, Some(false)),
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
            Arith::ShiftL => (y << 1, Some(y & 0x80 != 0)),
            Arith::ShiftR => (y >> 1, Some(y & 0x01 != 0))
        }
    }

    fn set_flag(&mut self, flag: bool) {
        self.regs[0xf] = if flag { 1 } else { 0 };
    }

    fn try_get_key<P>(&mut self, io: &mut P) -> Option<Byte> where P: Peripherals {
        let new_state = io.get_keys();

        let fresh_keys = new_state & !self.prev_key_state;
        self.prev_key_state = new_state;
        let idx = fresh_keys.trailing_zeros() as Byte;
        if idx < 16 { Some(idx) } else { None }
    }

    /// 16-bit LFSR a la https://en.wikipedia.org/wiki/Linear-feedback_shift_register
    fn next_random(&mut self) -> Byte {
        let lsb = self.rnd & 1 != 0;
        self.rnd >>= 1;
        if lsb { self.rnd ^= 0xb400 }
        self.rnd as Byte
    }

    pub fn step<P>(&mut self, io: &mut P) where P: Peripherals {
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
                let (z, flag) = CPU::arith(op, x, y);
                self.regs[vx as usize] = z;
                flag.map(|flag| { self.set_flag(flag); });
            },
            Op::LoadI(addr) => {
                self.addr = addr;
            },
            Op::AddI(vx) => {
                let addr = self.addr + self.regs[vx as usize] as u16;
                self.set_flag(addr > 0x0fff);
                self.addr = addr & 0x0fff;
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
                self.addr = (self.regs[vx as usize] as u16 & 0x0f) << 3;
            },
            Op::StoreBCD(vx) => {
                let x = self.regs[vx as usize];
                io.write_ram(self.addr, x / 100);
                io.write_ram(self.addr + 1, (x % 100) / 10);
                io.write_ram(self.addr + 2, x % 10);
            },
            Op::Save(vx) => {
                for i in 0..vx as usize +1 {
                    io.write_ram(self.addr + i as Addr, self.regs[i])
                }
            },
            Op::Restore(vx) => {
                for i in 0..vx as usize +1 {
                    self.regs[i] = io.read_ram(self.addr + i as Addr)
                }
            },
            Op::Draw(vx, vy, n) => {
                let mut collision = false;
                let xd = (self.regs[vx as usize]) & 0x3f;
                for i in 0..n {
                    let yd = (self.regs[vy as usize] + i) & 0x1f;
                    let dat = io.read_ram(self.addr + i as Addr);
                    let row = ((dat as ScreenRow) << 56) >> xd;

                    let old_row = io.get_pixel_row(yd);
                    let new_row = old_row ^ row;
                    collision |= old_row & row != 0;
                    io.set_pixel_row(yd, new_row);
                };
                io.redraw();
                self.set_flag(collision);
            },
            Op::ClearScr => {
                for y in 0..32 {
                    io.set_pixel_row(y, 0);
                }
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
                match self.prev_key {
                    None => {
                        if let Some(key) = self.try_get_key(io) {
                            self.regs[vx as usize] = key;
                            self.prev_key = Some(key);
                        }
                        self.pc -= 2;
                    },
                    Some(key) => {
                        let pressed = io.get_keys() & (1 << key) != 0;
                        if !pressed {
                            self.prev_key = None;
                        } else {
                            self.pc -= 2;
                        }
                    }
                }
            },
            Op::SetSound(vx) => {
                io.set_sound(self.regs[vx as usize]);
            },
        }
    }
}
