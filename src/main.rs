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
                    0b100 => {
                        unimplemented!()
                    }
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
                    0b100 => {
                        unimplemented!()
                    }
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
                    0b100 => {
                        unimplemented!()
                    }
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
            0b11 => {
                unimplemented!()
            }
            _ => unreachable!(),
        }
    }
    fn write_u32(&self, v: u32, emu: &mut Emulator) {
        if self.mo == 0b11 {
            // eax
            emu.write_reg(self.rm as usize, v);
        } else {
            // [eax], [eax]+disp, disp
            let addr = self.calc_memory_address(emu);
            emu.mem.write_u32(addr, v);
        }
    }
    fn read_u32(&self, emu: &mut Emulator) -> u32 {
        if self.mo == 0b11 {
            emu.read_reg(self.rm as usize)
        } else {
            let addr = self.calc_memory_address(emu);
            emu.mem.read_u32(addr)
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
    dbg!(d);
    if d >= 0 {
        emu.eip += d as u32;
    } else {
        emu.eip -= (-d) as u32;
    }
});
define_inst!(near_jump, emu, {
    let diff: i32 = emu.mem.read_i32(emu.eip + 1);
    let d = diff + 5;
    dbg!(d);
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
        for i in 0..8 {
            insts.insert(0xB8 + i, Arc::new(mov_r32_imm32));
        }
        insts.insert(0xE9, Arc::new(near_jump));
        insts.insert(0xEB, Arc::new(short_jump));

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
    fn exec(&mut self) {
        while self.eip < MEMORY_SIZE {
            eprintln!("eip: {}", self.eip);

            let opcode = self.mem.read_u8(self.eip);
            if let Some(inst) = self.insts.get(&opcode) {
                eprintln!("op: {:X}", opcode);
                let inst = Arc::clone(&inst);
                inst.exec(self);
            } else {
                eprintln!("op({}) not implemented", opcode);
                break;
            }

            if self.eip == 0x00 {
                eprintln!("end of program");
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
