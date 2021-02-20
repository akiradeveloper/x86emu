use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::Clap;
use std::collections::HashMap;
use std::sync::Arc;

const MEMORY_SIZE: u32 = 1 << 20; // 1MB

enum Disp {
    None,
    i8(i8),
    i32(i32),
}
struct ModRM {
    mo: u8,
    re: u8,
    rm: u8,
    sib: Option<u8>,
    disp: Disp,
}
impl ModRM {
    fn parse(emu: &mut Emulator) -> ModRM {
        let mut x = ModRM {
            mo: 0,
            re: 0,
            rm: 0,
            sib: None,
            disp: Disp::None,
        };
        let code = emu.mem.read_u8(emu.eip);
        x.mo = (code & 0b11000000) >> 6;
        x.re = (code & 0b00111000) >> 3;
        x.rm = (code & 0b00000111);
        emu.eip += 1;

        if (x.mo != 0b11 && x.rm == 0b100) {
            x.sib = Some(emu.mem.read_u8(emu.eip));
            emu.eip += 1;
        }

        // Mod(00) & RM(101) is a special case of disp32 (See Table 3.6)
        if x.mo == 0b10 || (x.mo == 0b00 && x.rm == 0b101) {
            x.disp = Disp::i32(emu.mem.read_i32(emu.eip));
            emu.eip += 4;
        } else if x.mo == 0b01 {
            x.disp = Disp::i8(emu.mem.read_i8(emu.eip));
            emu.eip += 1;
        }

        x
    }
    fn calc_memory_address(&self, emu: &Emulator) -> u32 {
        match self.mo {
            0b00 => {
                match self.rm {
                    0b100 => unimplemented!(),
                    0b101 => {
                        // disp32
                        if let Disp::i32(x) = self.disp {
                            x as u32
                        } else {
                            unreachable!()
                        }
                    }
                    _ => {
                        // [eax]
                        emu.read_reg(self.rm as usize)
                    }
                }
            }
            0b01 => {
                match self.rm {
                    0b100 => unimplemented!(),
                    _ => {
                        // [eax] + disp8
                        if let Disp::i8(x) = self.disp {
                            let base = emu.read_reg(self.rm as usize);
                            if x >= 0 {
                                base + x as u32
                            } else {
                                base - (-x) as u32
                            }
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
            0b10 => {
                match self.rm {
                    0b100 => unimplemented!(),
                    _ => {
                        // [eax] + disp32
                        if let Disp::i32(x) = self.disp {
                            let base = emu.read_reg(self.rm as usize);
                            if x >= 0 {
                                base + x as u32
                            } else {
                                base - (-x) as u32
                            }
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
            0b11 => unimplemented!(),
            _ => unreachable!(),
        }
    }
    fn write_u32(&self, v: u32, emu: &mut Emulator) {
        match self.mo {
            0b11 => {
                // eax
                emu.write_reg(self.rm as usize, v);
            }
            _ => {
                // [eax], [eax]+disp, disp
                let addr = self.calc_memory_address(emu);
                emu.mem.write_u32(addr, v);
            }
        }
    }
    fn read_u32(&self, emu: &mut Emulator) -> u32 {
        match self.mo {
            0b11 => emu.read_reg(self.rm as usize),
            _ => {
                let addr = self.calc_memory_address(emu);
                emu.mem.read_u32(addr)
            }
        }
    }
}

trait Instruction {
    fn exec(&self, emu: &mut Emulator);
}
macro_rules! define_inst {
    ($name:ident, $emu:ident, $code:block) => {
        struct $name;
        impl Instruction for $name {
            fn exec(&self, $emu: &mut Emulator) $code
        }
    }
}
define_inst!(mov_r32_imm32, emu, {
    let k = emu.mem.read_u8(emu.eip) - 0xB8;
    let v = emu.mem.read_u32(emu.eip + 1);
    emu.regs[k as usize] = v;
    emu.eip += 5;
});
define_inst!(short_jump, emu, {
    let diff: i8 = emu.mem.read_i8(emu.eip + 1);
    let d = diff + 2;
    if d >= 0 {
        emu.eip += d as u32;
    } else {
        emu.eip -= (-d) as u32;
    }
});
define_inst!(near_jump, emu, {
    let diff: i32 = emu.mem.read_i32(emu.eip + 1);
    let d = diff + 5;
    if d >= 0 {
        emu.eip += d as u32;
    } else {
        emu.eip -= (-d) as u32;
    }
});
define_inst!(mov_rm32_imm32, emu, {
    emu.eip += 1;
    let modrm = ModRM::parse(emu);
    let v = emu.mem.read_u32(emu.eip);
    emu.eip += 4;
    modrm.write_u32(v, emu);
});
define_inst!(mov_rm32_r32, emu, {
    emu.eip += 1;
    let modrm = ModRM::parse(emu);
    let v = emu.read_reg(modrm.re as usize);
    modrm.write_u32(v, emu);
});
define_inst!(mov_r32_rm32, emu, {
    emu.eip += 1;
    let modrm = ModRM::parse(emu);
    let v = modrm.read_u32(emu);
    emu.write_reg(modrm.re as usize, v);
});
define_inst!(add_rm32_r32, emu, {
    emu.eip += 1;
    let modrm = ModRM::parse(emu);
    let a = modrm.read_u32(emu);
    let b = emu.read_reg(modrm.re as usize);
    let c = a + b;
    modrm.write_u32(c, emu);
    // TODO eflags
});
define_inst!(cmp_r32_rm32, emu, {
    emu.eip += 1;
    let modrm = ModRM::parse(emu);
    let a = emu.read_reg(modrm.re as usize);
    let b = modrm.read_u32(emu);
    let c = a as u64 - b as u64;
    update_eflags(&mut emu.eflags, a, b, c);
});
define_inst!(code_83, emu, {
    emu.eip += 1;
    let modrm = ModRM::parse(emu);
    match modrm.re {
        0 => {
            // add_rm32_imm8
            let a = modrm.read_u32(emu);
            let b = emu.mem.read_i8(emu.eip) as u32;
            emu.eip += 1;
            let c = a as i64 + b as i64;
            modrm.write_u32(c as u32, emu);
            update_eflags(&mut emu.eflags, a, b, c as u64);
        }
        5 => {
            // sub_rm32_imm8
            let a = modrm.read_u32(emu);
            let b = emu.mem.read_i8(emu.eip) as u32;
            emu.eip += 1;
            let c = a as i64 - b as i64;
            modrm.write_u32(c as u32, emu);
            update_eflags(&mut emu.eflags, a, b, c as u64);
        }
        7 => {
            // cmp_rm32_imm8
            let a = modrm.read_u32(emu);
            let b = emu.mem.read_i8(emu.eip) as u32;
            emu.eip += 1;
            let c = a as i64 - b as i64;
            update_eflags(&mut emu.eflags, a, b, c as u64);
        }
        _ => unreachable!(),
    }
});
define_inst!(code_ff, emu, {
    emu.eip += 1;
    let modrm = ModRM::parse(emu);
    match modrm.re {
        0 => {
            // inc_rm32
            let a = modrm.read_u32(emu);
            modrm.write_u32(a + 1, emu);
        }
        _ => unimplemented!(),
    }
});
define_inst!(push_imm8, emu, {
    let v = emu.mem.read_u8(emu.eip + 1);
    emu.push(v as u32);
    emu.eip += 2;
});
define_inst!(push_imm32, emu, {
    let v = emu.mem.read_u32(emu.eip + 1);
    emu.push(v);
    emu.eip += 5;
});
define_inst!(push_r32, emu, {
    let reg = emu.mem.read_u8(emu.eip) - 0x50;
    let v = emu.read_reg(reg as usize);
    emu.push(v);
    emu.eip += 1;
});
define_inst!(pop_r32, emu, {
    let reg = emu.mem.read_u8(emu.eip) - 0x58;
    let v = emu.pop();
    emu.write_reg(reg as usize, v);
    emu.eip += 1;
});
define_inst!(call_rel32, emu, {
    let diff = emu.mem.read_i32(emu.eip + 1);
    // Push the address after call
    emu.push(emu.eip + 5);
    let d = diff + 5;
    if d >= 0 {
        emu.eip += d as u32;
    } else {
        emu.eip -= (-d) as u32;
    }
});
define_inst!(leave, emu, {
    // mov esp, ebp
    let ebp = emu.read_reg(REG::EBP as usize);
    emu.write_reg(REG::ESP as usize, ebp);
    // pop ebp
    let v = emu.pop();
    emu.write_reg(REG::EBP as usize, v);
    emu.eip += 1;
});
define_inst!(ret, emu, {
    emu.eip = emu.pop();
});
enum REG {
    EAX,
    ECX,
    EDX,
    EBX,
    ESP,
    EBP,
    ESI,
    EDI,
    COUNT,
}
struct Memory {
    v: Vec<u8>,
}
impl Memory {
    pub fn new(sz: u32) -> Self {
        Self {
            v: vec![0; sz as usize],
        }
    }
    fn load_bin(&mut self, bin: &[u8], at: usize) {
        let n = bin.len();
        let buf = &mut self.v[at..at + n];
        buf.copy_from_slice(&bin)
    }
    fn read_u8(&self, i: u32) -> u8 {
        let mut buf = &self.v[i as usize..];
        buf.read_u8().unwrap()
    }
    fn read_i8(&self, i: u32) -> i8 {
        let mut buf = &self.v[i as usize..];
        buf.read_i8().unwrap()
    }
    fn read_u32(&self, i: u32) -> u32 {
        let mut buf = &self.v[i as usize..];
        buf.read_u32::<LittleEndian>().unwrap()
    }
    fn read_i32(&self, i: u32) -> i32 {
        let mut buf = &self.v[i as usize..];
        buf.read_i32::<LittleEndian>().unwrap()
    }
    fn write_u32(&mut self, i: u32, v: u32) {
        let mut buf = &mut self.v[i as usize..];
        buf.write_u32::<LittleEndian>(v).unwrap()
    }
}
enum EFLAGS_SHIFT {
    CARRY = 1,
    ZERO = 6,
    SIGN = 7,
    OVERFLOW = 11,
}
fn set(eflags: &mut u32, shift: u32) {
    *eflags |= (1<<shift)
}
fn unset(eflags: &mut u32, shift: u32) {
    *eflags &= !(1<<shift)
}
fn update_eflags(out: &mut u32, x: u32, y: u32, z: u64) {
    use EFLAGS_SHIFT::*;

    let sign_x = x >> 31 > 0;
    let sign_y = y >> 31 > 0 ;
    let sign_z = (z >> 31) & 1 > 0;

    if z>>32 > 0 {
        set(out, CARRY as u32);
    } else {
        unset(out, CARRY as u32);
    }

    if z == 0 {
        set(out, ZERO as u32);
    } else {
        unset(out, ZERO as u32);
    }

    if sign_z {
        set(out, SIGN as u32);
    } else {
        unset(out, SIGN as u32);
    }

    if sign_x != sign_y && sign_x != sign_z {
        set(out, OVERFLOW as u32);
    } else {
        unset(out, OVERFLOW as u32);
    }
}
struct Emulator {
    regs: Vec<u32>,
    eflags: u32,
    eip: u32,

    mem: Memory,

    // code -> inst
    insts: HashMap<u8, Arc<dyn Instruction>>,
}
impl Emulator {
    fn new(mem_size: u32, eip: u32, esp: u32) -> Self {
        let mut insts: HashMap<u8, Arc<dyn Instruction>> = HashMap::new();

        insts.insert(0x01, Arc::new(add_rm32_r32));
        for i in 0..8 {
            insts.insert(0x50 + i, Arc::new(push_r32));
        }
        for i in 0..8 {
            insts.insert(0x58 + i, Arc::new(pop_r32));
        }
        insts.insert(0x68, Arc::new(push_imm32));
        insts.insert(0x6a, Arc::new(push_imm8));
        insts.insert(0x83, Arc::new(code_83));
        insts.insert(0x89, Arc::new(mov_rm32_r32));
        insts.insert(0x8B, Arc::new(mov_r32_rm32));
        for i in 0..8 {
            insts.insert(0xB8 + i, Arc::new(mov_r32_imm32));
        }
        insts.insert(0xC3, Arc::new(ret));
        insts.insert(0xC7, Arc::new(mov_rm32_imm32));
        insts.insert(0xC9, Arc::new(leave));
        insts.insert(0xE8, Arc::new(call_rel32));
        insts.insert(0xE9, Arc::new(near_jump));
        insts.insert(0xEB, Arc::new(short_jump));
        insts.insert(0xFF, Arc::new(code_ff));

        let mut x = Emulator {
            regs: vec![0; REG::COUNT as usize],
            eflags: 0,
            eip,
            mem: Memory::new(mem_size),
            insts,
        };
        x.regs[REG::ESP as usize] = esp;
        x
    }
    fn read_reg(&self, i: usize) -> u32 {
        self.regs[i]
    }
    fn write_reg(&mut self, i: usize, v: u32) {
        self.regs[i] = v;
    }
    fn push(&mut self, v: u32) {
        let new_esp = self.read_reg(REG::ESP as usize) - 4;
        self.write_reg(REG::ESP as usize, new_esp);
        self.mem.write_u32(new_esp, v);
    }
    fn pop(&mut self) -> u32 {
        let cur_esp = self.read_reg(REG::ESP as usize);
        let v = self.mem.read_u32(cur_esp);
        self.write_reg(REG::ESP as usize, cur_esp + 4);
        v
    }
    fn print_registers(&self) {
        eprintln!("EAX = {:X}", self.regs[REG::EAX as usize]);
        eprintln!("ECX = {:X}", self.regs[REG::ECX as usize]);
        eprintln!("EDX = {:X}", self.regs[REG::EDX as usize]);
        eprintln!("EBX = {:X}", self.regs[REG::EBX as usize]);
        eprintln!("ESP = {:X}", self.regs[REG::ESP as usize]);
        eprintln!("EBP = {:X}", self.regs[REG::EBP as usize]);
        eprintln!("ESI = {:X}", self.regs[REG::ESI as usize]);
        eprintln!("EDI = {:X}", self.regs[REG::EDI as usize]);
        eprintln!("EIP = {:X}", self.eip);
    }
    fn exec(&mut self) {
        let mut step = 0;
        while self.eip < MEMORY_SIZE {
            step += 1;
            eprintln!("----------");
            eprintln!("STEP {}", step);
            self.print_registers();

            let opcode = self.mem.read_u8(self.eip);
            if let Some(inst) = self.insts.get(&opcode) {
                eprintln!("op: {:X}", opcode);
                let inst = Arc::clone(&inst);
                inst.exec(self);
            } else {
                eprintln!("op({:X}) not implemented", opcode);
                break;
            }

            if self.eip == 0x00 {
                eprintln!("----------");
                eprintln!("END");
                self.print_registers();
                break;
            }
        }
    }
}
#[derive(Clap)]
struct Opts {
    bin_file: String,
}
fn main() {
    let opts = Opts::parse();

    let mut emu = Emulator::new(MEMORY_SIZE, 0x7c00, 0x7c00);

    let bin = std::fs::read(opts.bin_file).expect("failed to read program");
    emu.mem.load_bin(&bin, 0x7c00);

    emu.exec();
}
