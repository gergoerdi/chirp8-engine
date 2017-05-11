pub use prelude::*;
pub use opcodes::*;
pub use peripherals::Peripherals;

pub struct CPU {
    regs: [Byte; 16],
    addr: Addr,
    pc: Addr,
    stack: [Addr; 16],
    sp: usize,
}

impl CPU {
    pub fn new() -> CPU {
        CPU{ regs : [0; 16],
             addr: 0,
             pc: 0x200,
             stack: [0; 16],
             sp: 0
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
            Arith::Or => (x | y, None),
            Arith::And => (x & y, None),
            Arith::XOr => (x ^ y, None),
            Arith::Add => {
                let (z, f) = u8::overflowing_add(x, y);
                (z, Some(f))
            },
            Arith::Sub => {
                let (z, f) = u8::overflowing_sub(x, y);
                (z, Some(f))
            },
            Arith::SubFlip => {
                let (z, f) = u8::overflowing_sub(y, x);
                (z, Some(f))
            },
            Arith::ShiftL => (x << 1, Some(x & 0x80 != 0)),
            Arith::ShiftR => (x >> 1, Some(x & 0x01 != 0))
        }
    }

    fn set_flag(&mut self, flag: bool) {
        self.regs[0xf] = if flag { 1 } else { 0 };
    }

    fn wait_key<P>(&self, io: &mut P) -> Byte where P: Peripherals {
        let mut old_state = io.get_keys();

        while io.keep_running() {
            let new_state = io.get_keys();

            let fresh_keys = new_state & !old_state;
            let idx = fresh_keys.trailing_zeros() as Byte;
            if idx < 16 { return idx };

            old_state &= new_state;
        }

        0
    }

    #[inline(never)]
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
               self.regs[vx as usize] = io.get_timer();
            },
            Op::SetTimer(vx) => {
                io.set_timer(self.regs[vx as usize]);
            },
            Op::JumpV0(addr) => {
                self.pc = addr + self.regs[0] as Addr;
            },
            Op::Random(vx, mask) => {
                let rnd = io.get_random();
                self.regs[vx as usize] = rnd & mask;
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
                for i in 0..n {
                    let yd = (self.regs[vy as usize] + i) & 0x1f;
                    let mut row = io.read_ram(self.addr + i as Addr);
                    for j in 0..8 {
                        let xd = (self.regs[vx as usize] + j) & 0x3f;

                        let old_pixel = io.get_pixel(xd, yd);
                        let new_pixel = (row & (1 << 7)) != 0;
                        row <<= 1;
                        collision |= old_pixel && new_pixel;
                        io.set_pixel(xd, yd, old_pixel != new_pixel);
                    }
                };
                io.redraw();
                self.set_flag(collision);
            },
            Op::ClearScr => {
                for x in 0..64 {
                    for y in 0..32 {
                        io.set_pixel(x, y, false);
                    }
                }
            },
            Op::SkipKey(cond, vx) => {
                let pressed = io.get_keys() & (1 << vx) != 0;
                let target = match cond {
                    Cmp::Eq => true,
                    Cmp::NEq => false
                };
                if pressed == target {
                    self.pc += 2;
                }
            },
            Op::WaitKey(vx) => {
                self.regs[vx as usize] = self.wait_key(io);
            },
            Op::SetSound(vx) => {
                io.set_sound(self.regs[vx as usize]);
            },
        }
    }
}
