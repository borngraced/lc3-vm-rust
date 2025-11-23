#![allow(non_camel_case_types, unused)]
mod instructions;
mod trap;

use nix::{
    libc::{self, ECHO, ICANON, TCSANOW, tcgetattr, tcsetattr, termios},
    sys::{
        select::{FdSet, select},
        time::{TimeVal, TimeValLike},
    },
};

use crate::{
    instructions::{
        AddAndInstruction, AddressingMode, JsrInstruction, LdInstruction, LdiInstruction,
        LdrInstruction, LeaInstruction, NotInstruction, StInstruction, StiInstruction,
        StrInstruction,
    },
    trap::{TrapCode, trap},
};

use std::{
    env::args,
    ffi::IntoStringError,
    fs::File,
    io::{Read, Write, stdin, stdout},
    os::fd::BorrowedFd,
    path::PathBuf,
    thread::sleep,
    time::Duration,
};

const ORIGIN_MAX: usize = 2;
const MEMORY_MAX: usize = 1 << 16;
const MR_KBSR: usize = 0xFE00;
const MR_KBDR: usize = 0xFE02;

#[derive(Debug, Eq, PartialEq)]
enum Register {
    R_R0 = 0,
    R_R1,
    R_R2,
    R_R3,
    R_R4,
    R_R5,
    R_R6,
    R_R7,
    R_PC,
    R_COND,
    R_COUNT,
}

impl Register {
    fn as_usize(&self) -> usize {
        match self {
            Register::R_R0 => 0,
            Register::R_R1 => 1,
            Register::R_R2 => 2,
            Register::R_R3 => 3,
            Register::R_R4 => 4,
            Register::R_R5 => 5,
            Register::R_R6 => 6,
            Register::R_R7 => 7,
            Register::R_PC => 8,
            Register::R_COND => 9,
            Register::R_COUNT => 10,
        }
    }
}

impl From<u8> for Register {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::R_R0,
            1 => Self::R_R1,
            2 => Self::R_R2,
            3 => Self::R_R3,
            4 => Self::R_R4,
            5 => Self::R_R5,
            6 => Self::R_R6,
            7 => Self::R_R7,
            8 => Self::R_PC,
            9 => Self::R_COND,
            10 => Self::R_COUNT,
            _ => panic!("invalid register"),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum OpCode {
    OP_BR = 0,
    OP_ADD,  // add
    OP_LD,   // load
    OP_ST,   // store
    OP_JSR,  // jump register
    OP_AND,  // bitwise AND
    OP_LDR,  // load register
    OP_STR,  // store register
    OP_RTI,  // unused
    OP_NOT,  // bitwise not
    OP_LDI,  // load indirect
    OP_STI,  // store indirect
    OP_JMP,  // jump
    OP_RES,  // reserved unused
    OP_LEA,  // load effective address
    OP_TRAP, // execute trap
    OP_BAD,
}

impl From<usize> for OpCode {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::OP_BR,
            1 => Self::OP_ADD,
            2 => Self::OP_LD,
            3 => Self::OP_ST,
            4 => Self::OP_JSR,
            5 => Self::OP_AND,
            6 => Self::OP_LDR,
            7 => Self::OP_STR,
            8 => Self::OP_RTI,
            9 => Self::OP_NOT,
            10 => Self::OP_LDI,
            11 => Self::OP_STI,
            12 => Self::OP_JMP,
            13 => Self::OP_RES,
            14 => Self::OP_LEA,
            15 => Self::OP_TRAP,
            _ => Self::OP_BAD,
        }
    }
}

enum ConditionFlag {
    FL_POS = 1 << 0,
    FL_ZRO = 1 << 1,
    FL_NEG = 1 << 2,
}

fn swap16(v: u16) -> u16 {
    return (v << 8) | (v >> 8);
}

fn sign_extend(mut x: u16, bit_count: u8) -> u16 {
    if (((x >> (bit_count - 1)) & 1) == 1) {
        x |= (0xFFFF << bit_count)
    }

    x
}

fn check_key() -> bool {
    let mut readfds = FdSet::new();
    readfds.insert(unsafe { BorrowedFd::borrow_raw(libc::STDIN_FILENO) });

    let mut timeout = TimeVal::milliseconds(0);

    unsafe {
        match select(1, Some(&mut readfds), None, None, Some(&mut timeout)) {
            Ok(n) if n > 0 => readfds.contains(BorrowedFd::borrow_raw(libc::STDIN_FILENO)),
            Ok(_) => false,  // n == 0 → timeout
            Err(_) => false, // error → assume no key
        }
    }
}

struct Svm {
    reg: Vec<u16>,
    mem: Vec<u16>,
    image_path: PathBuf,
    current_instr: Option<u16>,
}

impl Svm {
    fn new(image_path: PathBuf) -> Self {
        let mut reg = vec![0u16; 12];
        reg[Register::R_COND.as_usize()] = ConditionFlag::FL_ZRO as u16;

        // set the pc to starting position 0x3000 is default;
        const PC_START: u16 = 0x3000;
        reg[Register::R_PC.as_usize()] = PC_START;

        Self {
            reg,
            mem: vec![0u16; MEMORY_MAX],
            image_path,
            current_instr: None,
        }
    }

    fn read_image(&mut self) {
        println!("reading image: {:?}", self.image_path);
        let mut file = File::open(&self.image_path).expect("unable to open file");
        let mut buffer = Vec::with_capacity(MEMORY_MAX);
        file.read_to_end(&mut buffer).expect("unable to read file");

        let origin = swap16(u16::from_le_bytes(buffer[..2].try_into().unwrap()));
        let words = &buffer[2..];
        if words.len() % 2 != 0 {
            println!("odd bytes len! last byte ignored");
        }

        for (i, chunk) in words.chunks_exact(2).enumerate() {
            let addr_val = swap16(u16::from_le_bytes(chunk.try_into().unwrap()));
            let idx = origin as usize + i;
            self.mem[idx] = addr_val;
        }

        println!(
            "loaded {} words from {:?} : origin {:#06x}",
            words.len() / 2,
            self.image_path,
            origin
        );
    }

    pub fn mem_read(&mut self, addr: usize) -> u16 {
        if addr == MR_KBSR {
            if check_key() {
                self.mem[MR_KBSR] = 1 << 15;
                self.mem[MR_KBDR] = stdin()
                    .bytes()
                    .next()
                    .and_then(|r| r.ok())
                    .expect("unable to read byte") as u16;
            } else {
                self.mem[MR_KBSR] = 0;
            }
        }

        self.mem[addr]
    }

    pub fn mem_write(&mut self, addr: usize, val: u16) {
        self.mem[addr] = val;
    }

    fn update_flags(&mut self, r: Register) {
        let val = self.reg[r.as_usize()];
        if val == 0 {
            self.reg[Register::R_COND.as_usize()] = ConditionFlag::FL_ZRO as u16;
        } else if (val >> 15) == 1 {
            self.reg[Register::R_COND.as_usize()] = ConditionFlag::FL_NEG as u16;
        } else {
            self.reg[Register::R_COND.as_usize()] = ConditionFlag::FL_POS as u16;
        }
    }

    fn pc_counter(&mut self) -> OpCode {
        let instr = self.mem_read(self.reg[Register::R_PC.as_usize()] as usize);
        self.reg[Register::R_PC.as_usize()] += 1;

        self.current_instr = Some(instr);
        OpCode::from((instr >> 12) as usize)
    }

    fn current_instr(&self) -> u16 {
        self.current_instr
            .expect("this should always return a valid instruction.")
    }

    fn add(&mut self) {
        let instr = AddAndInstruction::new(self.current_instr());
        let source_register_1 = self.reg[instr.source_register_1.as_usize()];

        match &instr.mode {
            AddressingMode::Reg { source_register_2 } => {
                self.reg[instr.destination_register.as_usize()] =
                    self.reg[source_register_2.as_usize()].wrapping_add(source_register_1)
            }
            AddressingMode::Imm(value) => {
                self.reg[instr.destination_register.as_usize()] =
                    source_register_1.wrapping_add(*value)
            }
        }

        self.update_flags(instr.destination_register);
    }

    fn and(&mut self) {
        let instr = AddAndInstruction::new(self.current_instr());

        match &instr.mode {
            AddressingMode::Reg { source_register_2 } => {
                self.reg[instr.destination_register.as_usize()] = self.reg
                    [instr.source_register_1.as_usize()]
                    & self.reg[source_register_2.as_usize()]
            }
            AddressingMode::Imm(value) => {
                self.reg[instr.destination_register.as_usize()] =
                    self.reg[instr.source_register_1.as_usize()] & *value
            }
        }

        self.update_flags(instr.destination_register);
    }

    fn br(&mut self) {
        let instr = self.current_instr();
        let pc_offset = sign_extend(instr & 0x1FF, 9);

        let npz = (instr >> 9) & 0b111;

        if npz & self.reg[Register::R_COND.as_usize()] != 0 {
            self.reg[Register::R_PC.as_usize()] =
                self.reg[Register::R_PC.as_usize()].wrapping_add(pc_offset);
        }
    }

    fn ldi(&mut self) {
        let instr = LdiInstruction::new(self.current_instr());
        let addr = self
            .mem_read((self.reg[Register::R_PC.as_usize()].wrapping_add(instr.pc_offset)) as usize);
        self.reg[instr.destination_register.as_usize()] = self.mem_read(addr as usize);

        self.update_flags(instr.destination_register);
    }

    fn ld(&mut self) {
        let instr = LdInstruction::new(self.current_instr());

        self.reg[instr.destination_register.as_usize()] = self.mem_read(
            (self.reg[Register::R_PC.as_usize()].wrapping_add(instr.pc_offset) as usize) as usize,
        );
        self.update_flags(instr.destination_register);
    }

    fn ldr(&mut self) {
        let instr = LdrInstruction::new(self.current_instr());

        self.reg[instr.destination_register.as_usize()] = self.mem_read(
            (self.reg[instr.base_register.as_usize()]).wrapping_add(instr.offset) as usize,
        );

        self.update_flags(instr.destination_register);
    }

    fn st(&mut self) {
        let instr = StInstruction::new(self.current_instr());
        let addr = self.reg[Register::R_PC.as_usize()].wrapping_add(instr.pc_offset) as usize;

        self.mem_write(addr as usize, self.reg[instr.source_register.as_usize()]);
    }

    fn sti(&mut self) {
        let instr = StiInstruction::new(self.current_instr());
        let addr = self
            .mem_read((self.reg[Register::R_PC.as_usize()].wrapping_add(instr.pc_offset)) as usize);
        self.mem_write(addr as usize, self.reg[instr.source_register.as_usize()]);
    }

    fn str(&mut self) {
        let instr = StrInstruction::new(self.current_instr());
        let addr = (self.reg[instr.base_register.as_usize()].wrapping_add(instr.offset)) as usize;
        let value = self.reg[instr.source_register.as_usize()];

        self.mem_write(addr as usize, value);
    }

    fn lea(&mut self) {
        let instr = LeaInstruction::new(self.current_instr());

        self.reg[instr.destination_register.as_usize()] =
            self.reg[Register::R_PC.as_usize()].wrapping_add(instr.pc_offset);

        self.update_flags(instr.destination_register);
    }

    fn not(&mut self) {
        let instr = NotInstruction::new(self.current_instr());

        self.reg[instr.destination_register.as_usize()] =
            !self.reg[instr.source_register.as_usize()];

        self.update_flags(instr.destination_register);
    }

    fn jmp(&mut self) {
        let base_register = ((self.current_instr() >> 6) & 0x7) as usize;
        if base_register == Register::R_R7.as_usize() {
            self.reg[Register::R_PC.as_usize()] = self.reg[Register::R_R7.as_usize()];
            return;
        }

        self.reg[Register::R_PC.as_usize()] = self.reg[base_register];
    }

    fn jsr(&mut self) {
        let instr = JsrInstruction::new(self.current_instr());

        let pc = self.reg[Register::R_PC.as_usize()];

        self.reg[Register::R_R7.as_usize()] = pc;

        if instr.bit_11 == 1 {
            self.reg[Register::R_PC.as_usize()] = pc.wrapping_add(instr.offset);
        } else {
            self.reg[Register::R_PC.as_usize()] = self.reg[instr.base_register.as_usize()];
        }
    }
}

pub fn disable_input_buffering() {
    unsafe {
        let mut term: termios = std::mem::zeroed();
        tcgetattr(libc::STDIN_FILENO, &mut term);
        term.c_lflag &= !(ICANON | ECHO);
        tcsetattr(libc::STDIN_FILENO, TCSANOW, &term);
    }
}

pub fn restore_input_buffering() {
    unsafe {
        let mut term: termios = std::mem::zeroed();
        tcgetattr(libc::STDIN_FILENO, &mut term);
        term.c_lflag |= ICANON | ECHO;
        tcsetattr(libc::STDIN_FILENO, TCSANOW, &term);
    }
}

fn handle_signal_interrupt() {}

fn main() {
    // ctrlc::set_handler(move || {
    //     restore_input_buffering();
    // });

    disable_input_buffering();

    let mut args = args().nth(1);
    let Some(path) = args else {
        panic!("lc3 obj [image-file1]...\n");
    };

    let mut svm = Svm::new(path.into());
    svm.read_image();

    loop {
        let op_code = svm.pc_counter();
        // println!(
        //     "executing op_code: {:?} with instr: {:#x}",
        //     op_code,
        //     svm.current_instr()
        // );

        // sleep(Duration::from_secs(1));

        match op_code {
            OpCode::OP_BR => svm.br(),
            OpCode::OP_ADD => svm.add(),
            OpCode::OP_AND => svm.and(),
            OpCode::OP_LD => svm.ld(),
            OpCode::OP_LDI => svm.ldi(),
            OpCode::OP_LDR => svm.ldr(),
            OpCode::OP_ST => svm.st(),
            OpCode::OP_STI => svm.sti(),
            OpCode::OP_STR => svm.str(),
            OpCode::OP_LEA => svm.lea(),
            OpCode::OP_NOT => svm.not(),
            OpCode::OP_JMP => svm.jmp(),
            OpCode::OP_JSR => svm.jsr(),

            OpCode::OP_TRAP => {
                if let TrapCode::HALT = trap(&mut svm) {
                    break;
                }
            }

            _ => panic!("unimplemented op_code"),
        }
    }

    restore_input_buffering();
}
