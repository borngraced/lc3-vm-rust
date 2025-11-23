//! https://www.jmeiners.com/lc3-vm/supplies/lc3-isa.pdf
use crate::{MEMORY_MAX, OpCode, Register, sign_extend};

#[derive(Debug, Eq, PartialEq)]
pub enum AddressingMode {
    Reg { source_register_2: Register },
    Imm(u16),
}

/// ADD Instruction
#[derive(Debug)]
pub struct AddAndInstruction {
    pub destination_register: Register,
    pub source_register_1: Register,
    pub mode: AddressingMode,
}

impl AddAndInstruction {
    pub fn new(instruction: u16) -> Self {
        let destination_register = Register::from(((instruction >> 9) & 0x7) as u8);
        let source_register_1 = Register::from(((instruction >> 6) & 0x7) as u8);
        let addressing_mode = (instruction >> 5) & 0x1;
        let mode = if addressing_mode == 0 {
            AddressingMode::Reg {
                source_register_2: Register::from((instruction & 0x7) as u8),
            }
        } else {
            AddressingMode::Imm(sign_extend(instruction & 0x1F, 5))
        };

        Self {
            destination_register,
            source_register_1,
            mode,
        }
    }
}

#[test]
fn test_add_instruction_register_addressing_mode() {
    const INSTRUCTION: u16 = 0xA74A;

    let add = AddAndInstruction::new(INSTRUCTION);

    dbg!(&add);
    assert_eq!(Register::R_R3, add.destination_register);
    assert_eq!(add.source_register_1, add.source_register_1);
    assert_eq!(
        add.mode,
        AddressingMode::Reg {
            source_register_2: Register::R_R2
        }
    );
}

// LDI instruction
/// [15     12][11   9][8         0]
///   OP_CODE     DR     pc offset9
///
#[derive(Debug)]
pub struct LdiInstruction {
    pub destination_register: Register,
    pub pc_offset: u16,
}

impl LdiInstruction {
    pub fn new(instr: u16) -> Self {
        let destination_register = Register::from(((instr >> 9) & 0x7) as u8);
        let pc_offset = sign_extend(instr & 0x1FF, 9);

        Self {
            destination_register,
            pc_offset,
        }
    }
}

#[test]
fn test_ldi_instruction_initializer() {
    const INSTRUCTION: u16 = 0xA6D5;
    let ldi = LdiInstruction::new(INSTRUCTION);

    dbg!(&ldi);
    assert_eq!(Register::R_R3, ldi.destination_register);
    assert_eq!(213, ldi.pc_offset);
}

#[derive(Debug, Eq, PartialEq)]
pub struct LdInstruction {
    pub destination_register: Register,
    pub pc_offset: u16,
}

impl LdInstruction {
    pub fn new(instr: u16) -> Self {
        let destination_register = Register::from(((instr >> 9) & 0x7) as u8);
        let pc_offset = sign_extend(instr & 0x1FF, 9);

        Self {
            destination_register,
            pc_offset,
        }
    }
}

#[test]
fn test_ld_instruction_initializer() {
    // 0010     001 000000011
    // opcode   DR(Register 1)    pc_offset(3)
    const INSTRUCTION: u16 = 0x2203;
    let ld = LdInstruction::new(INSTRUCTION);

    dbg!(&ld);
    assert_eq!(Register::R_R1, ld.destination_register);
    assert_eq!(3, ld.pc_offset);
}

#[derive(Debug, Eq, PartialEq)]
pub struct LdrInstruction {
    pub destination_register: Register,
    pub base_register: Register,
    pub offset: u16,
}

impl LdrInstruction {
    pub fn new(instr: u16) -> Self {
        let destination_register = Register::from(((instr >> 9) & 0x7) as u8);
        let base_register = Register::from(((instr >> 6) & 0x7) as u8);
        let offset = sign_extend(instr & 0x3F, 6);

        Self {
            destination_register,
            base_register,
            offset,
        }
    }
}

#[test]
fn test_ldr_instruction_initializer() {
    const INSTRUCTION: u16 = 0x628F;
    let ldr = LdrInstruction::new(INSTRUCTION);

    dbg!(&ldr);
    assert_eq!(Register::R_R1, ldr.destination_register);
    assert_eq!(Register::R_R2, ldr.base_register);
    assert_eq!(15, ldr.offset);
}

#[derive(Debug, Eq, PartialEq)]
pub struct StInstruction {
    pub source_register: Register,
    pub pc_offset: u16,
}

impl StInstruction {
    pub fn new(instr: u16) -> Self {
        let source_register = Register::from(((instr >> 9) & 0x7) as u8);
        let pc_offset = sign_extend(instr & 0x1FF, 9);

        Self {
            source_register,
            pc_offset,
        }
    }
}

#[test]
fn test_st_instruction_initializer() {
    // 0011     010 000001010
    // opcode   SR(Register 2)    pc_offset(10)
    const INSTRUCTION: u16 = 0x340A;
    let st = StInstruction::new(INSTRUCTION);

    dbg!(&st);
    assert_eq!(Register::R_R2, st.source_register);
    assert_eq!(10, st.pc_offset);
}

#[derive(Debug, Eq, PartialEq)]
pub struct StiInstruction {
    pub source_register: Register,
    pub pc_offset: u16,
}

impl StiInstruction {
    pub fn new(instr: u16) -> Self {
        let source_register = Register::from(((instr >> 9) & 0x7) as u8);
        let pc_offset = sign_extend(instr & 0x1FF, 9);

        Self {
            source_register,
            pc_offset,
        }
    }
}

#[test]
fn test_sti_instruction_initializer() {
    // 1011     010 000001010
    // opcode   SR(Register 2)    pc_offset(10)
    const INSTRUCTION: u16 = 0x340A;
    let st = StiInstruction::new(INSTRUCTION);

    dbg!(&st);
    assert_eq!(Register::R_R2, st.source_register);
    assert_eq!(10, st.pc_offset);
}

#[derive(Debug, Eq, PartialEq)]
pub struct StrInstruction {
    pub source_register: Register,
    pub base_register: Register,
    pub offset: u16,
}

impl StrInstruction {
    pub fn new(instr: u16) -> Self {
        let source_register = Register::from(((instr >> 9) & 0x7) as u8);
        let base_register = Register::from(((instr >> 6) & 0x7) as u8);
        let offset = sign_extend(instr & 0x3F, 6);

        Self {
            source_register,
            base_register,
            offset,
        }
    }
}

#[test]
fn test_str_instruction_initializer() {
    const INSTRUCTION: u16 = 0x744D;
    let str = StrInstruction::new(INSTRUCTION);

    dbg!(&str);
    assert_eq!(Register::R_R2, str.source_register);
    assert_eq!(Register::R_R1, str.base_register);
    assert_eq!(13, str.offset);
}

#[derive(Debug, Eq, PartialEq)]
pub struct LeaInstruction {
    pub destination_register: Register,
    pub pc_offset: u16,
}

impl LeaInstruction {
    pub fn new(instr: u16) -> Self {
        let destination_register = Register::from(((instr >> 9) & 0x7) as u8);
        let pc_offset = sign_extend(instr & 0x1FF, 9);

        Self {
            destination_register,
            pc_offset,
        }
    }
}

#[test]
fn test_lea_instruction_initializer() {
    const INSTRUCTION: u16 = 0xE20F;
    let lea = LeaInstruction::new(INSTRUCTION);

    dbg!(&lea);
    assert_eq!(Register::R_R1, lea.destination_register);
    assert_eq!(15, lea.pc_offset);
}

#[derive(Debug, Eq, PartialEq)]
pub struct NotInstruction {
    pub destination_register: Register,
    pub source_register: Register,
}

impl NotInstruction {
    pub fn new(instr: u16) -> Self {
        let destination_register = Register::from(((instr >> 9) & 0x7) as u8);
        let source_register = Register::from(((instr >> 6) & 0x7) as u8);

        Self {
            destination_register,
            source_register,
        }
    }
}

#[test]
fn test_not_instruction_initializer() {
    const INSTRUCTION: u16 = 0x9283;
    let not = NotInstruction::new(INSTRUCTION);

    dbg!(&not);
    assert_eq!(Register::R_R1, not.destination_register);
    assert_eq!(Register::R_R2, not.source_register);
}

#[derive(Debug, Eq, PartialEq)]
pub struct JsrInstruction {
    pub offset: u16,
    // bit_11 should be 0 for JSR instruction and 1 for JSRR.
    pub bit_11: u8,
    // jsrr
    pub base_register: Register,
}

impl JsrInstruction {
    pub fn new(instr: u16) -> Self {
        let bit_11 = ((instr >> 11) & 1) as u8; // won't overflow believe xddd;
        let offset = sign_extend((instr & 0x7FF), 11);
        let base_register = Register::from(((instr >> 6) & 0x7) as u8);

        Self {
            bit_11,
            offset,
            base_register,
        }
    }
}

#[test]
fn test_jsr_instruction_initialiazier() {
    const INSTRUCTION: u16 = 0x400F;
    let jsr = JsrInstruction::new(INSTRUCTION);

    dbg!(&jsr);
    // bit_11 should be 0 for JSR instruction
    assert_eq!(0, jsr.bit_11);
    assert_eq!(15, jsr.offset);
}
