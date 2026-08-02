//! https://www.jmeiners.com/lc3-vm/supplies/lc3-isa.pdf
use crate::{Register, sign_extend};
use packed_bits::packed_bits;

/// Generates a `destination_register + 9-bit pc_offset` instruction that
/// decodes from the shared [`OffsetInstr`] layout (LD, LDI, LEA, ST, STI).
macro_rules! offset_instr {
    ($doc:literal, $name:ident, $reg_field:ident) => {
        #[doc = $doc]
        #[derive(Debug, Eq, PartialEq)]
        pub struct $name {
            pub $reg_field: Register,
            pub pc_offset: u16,
        }

        impl $name {
            pub fn new(instr: u16) -> Self {
                let decoded = OffsetInstr::from_raw(instr);
                Self {
                    $reg_field: Register::from(decoded.reg() as u8),
                    pc_offset: sign_extend(decoded.pc_offset(), 9),
                }
            }
        }
    };
}

/// Generates a `register + base_register + 6-bit offset` instruction that
/// decodes from the shared [`BaseOffsetInstr`] layout (LDR, STR).
macro_rules! base_offset_instr {
    ($doc:literal, $name:ident, $reg_field:ident) => {
        #[doc = $doc]
        #[derive(Debug, Eq, PartialEq)]
        pub struct $name {
            pub $reg_field: Register,
            pub base_register: Register,
            pub offset: u16,
        }

        impl $name {
            pub fn new(instr: u16) -> Self {
                let decoded = BaseOffsetInstr::from_raw(instr);
                Self {
                    $reg_field: Register::from(decoded.reg() as u8),
                    base_register: Register::from(decoded.baser() as u8),
                    offset: sign_extend(decoded.offset(), 6),
                }
            }
        }
    };
}

//
// packed_bits layouts (fields are packed LSB-first; opcode is the MSB field)
//

/// ADD and AND share the same layout:
/// `opcode[15:12] dr[11:9] sr1[8:6] imm[5] sr2/imm5[4:0]`
/// SR2 (register mode) or imm5 (immediate mode)
/// 0 = register mode, 1 = immediate mode
packed_bits!(
    AddInstr: u16 { value: 5, imm: 1, sr1: 3, dr: 3, opcode: 4, }
);

/// `opcode[15:12] reg[11:9] pcoffset9[8:0]`
///
/// Shared layout for LD, LDI, LEA (DR) and ST, STI (SR):
/// all pack a 3-bit register at bits `[11:9]` plus a 9-bit pc offset.
packed_bits!(
    OffsetInstr: u16 { pc_offset: 9, reg: 3, opcode: 4, }
);

/// `opcode[15:12] reg[11:9] baser[8:6] offset6[5:0]`
///
/// Shared layout for LDR (DR) and STR (SR).
packed_bits!(
    BaseOffsetInstr: u16 { offset: 6, baser: 3, reg: 3, opcode: 4, }
);

/// `opcode[15:12] dr[11:9] sr[8:6] 000[5:0]`
packed_bits!(
    NotInstr: u16 { unused: 6, sr: 3, dr: 3, opcode: 4, }
);

/// `opcode[15:12] jsr[11] pcoffset11[10:0]`
/// In JSRR mode the base register lives in bits `[8:6]`, i.e. bits 6..8 of the
/// `offset` field, so it is derived from `offset`.
packed_bits!(
    JsrInstr: u16 { offset: 11, jsr: 1, opcode: 4, }
);

/// `opcode[15:12] nzp[11:9] pcoffset9[8:0]`
packed_bits!(
    BrInstr: u16 { pc_offset: 9, nzp: 3, opcode: 4, }
);

/// `opcode[15:12] 000[11:9] baser[8:6] 000000`
packed_bits!(
    JmpInstr: u16 { unused: 6, baser: 3, pad: 3, opcode: 4, }
);

//
// Instruction structs
//

offset_instr!("LDI instruction", LdiInstruction, destination_register);
offset_instr!("LD instruction.", LdInstruction, destination_register);
offset_instr!("ST instruction.", StInstruction, source_register);
offset_instr!("STI instruction.", StiInstruction, source_register);
offset_instr!("LEA instruction.", LeaInstruction, destination_register);
base_offset_instr!("LDR instruction.", LdrInstruction, destination_register);
base_offset_instr!("STR instruction.", StrInstruction, source_register);

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
        let instr = AddInstr::from_raw(instruction);
        let mode = if instr.imm() == 0 {
            AddressingMode::Reg {
                source_register_2: Register::from(instr.value() as u8),
            }
        } else {
            AddressingMode::Imm(sign_extend(instr.value(), 5))
        };

        Self {
            destination_register: Register::from(instr.dr() as u8),
            source_register_1: Register::from(instr.sr1() as u8),
            mode,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct NotInstruction {
    pub destination_register: Register,
    pub source_register: Register,
}

impl NotInstruction {
    pub fn new(instr: u16) -> Self {
        let decoded = NotInstr::from_raw(instr);
        Self {
            destination_register: Register::from(decoded.dr() as u8),
            source_register: Register::from(decoded.sr() as u8),
        }
    }
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
        let decoded = JsrInstr::from_raw(instr);
        Self {
            bit_11: decoded.jsr() as u8,
            offset: sign_extend(decoded.offset(), 11),
            base_register: Register::from(((decoded.offset() >> 6) & 0x7) as u8),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OpCode, Register};

    #[test]
    fn test_packed_bits_add_instr_imm_addr_mode() {
        const INSTRUCTION: u16 = 0x1465; // ADD R2, R1, imm5(5)
        let add = AddInstr::from_raw(INSTRUCTION);

        dbg!(&add);
        assert_eq!(OpCode::OP_ADD, OpCode::from(add.opcode() as usize));
        assert_eq!(Register::R_R2, (add.dr() as u8).into());
        assert_eq!(Register::R_R1, (add.sr1() as u8).into());
        assert_eq!(1, add.imm());
        assert_eq!(5, add.value());
    }

    #[test]
    fn test_add_instruction_register_addressing_mode() {
        const INSTRUCTION: u16 = 0x1042; // ADD R0, R1, R2
        let add = AddAndInstruction::new(INSTRUCTION);

        dbg!(&add);
        assert_eq!(Register::R_R0, add.destination_register);
        assert_eq!(Register::R_R1, add.source_register_1);
        assert_eq!(
            add.mode,
            AddressingMode::Reg {
                source_register_2: Register::R_R2
            }
        );
    }

    #[test]
    fn test_add_instruction_immediate_addressing_mode() {
        const INSTRUCTION: u16 = 0x1465; // ADD dst=R2, sr1=R2, imm5=5
        let add = AddAndInstruction::new(INSTRUCTION);

        dbg!(&add);
        assert_eq!(Register::R_R2, add.destination_register);
        assert_eq!(Register::R_R1, add.source_register_1);
        assert_eq!(add.mode, AddressingMode::Imm(5));
    }

    #[test]
    fn test_ldi_instruction_initializer() {
        const INSTRUCTION: u16 = 0xA6D5;
        let ldi = OffsetInstr::from_raw(INSTRUCTION);
        dbg!(&ldi);

        assert_eq!(Register::R_R3, (ldi.reg() as u8).into());
        assert_eq!(213, sign_extend(ldi.pc_offset(), 9));
    }

    #[test]
    fn test_ldi_instruction_new() {
        const INSTRUCTION: u16 = 0xA6D5;
        let ldi = LdiInstruction::new(INSTRUCTION);

        assert_eq!(Register::R_R3, ldi.destination_register);
        assert_eq!(213, ldi.pc_offset);
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

    #[test]
    fn test_ldr_instruction_initializer() {
        const INSTRUCTION: u16 = 0x628F;
        let ldr = LdrInstruction::new(INSTRUCTION);

        dbg!(&ldr);
        assert_eq!(Register::R_R1, ldr.destination_register);
        assert_eq!(Register::R_R2, ldr.base_register);
        assert_eq!(15, ldr.offset);
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

    #[test]
    fn test_str_instruction_initializer() {
        const INSTRUCTION: u16 = 0x744D;
        let str = StrInstruction::new(INSTRUCTION);

        dbg!(&str);
        assert_eq!(Register::R_R2, str.source_register);
        assert_eq!(Register::R_R1, str.base_register);
        assert_eq!(13, str.offset);
    }

    #[test]
    fn test_lea_instruction_initializer() {
        const INSTRUCTION: u16 = 0xE20F;
        let lea = LeaInstruction::new(INSTRUCTION);

        dbg!(&lea);
        assert_eq!(Register::R_R1, lea.destination_register);
        assert_eq!(15, lea.pc_offset);
    }

    #[test]
    fn test_not_instruction_initializer() {
        const INSTRUCTION: u16 = 0x9283;
        let not = NotInstruction::new(INSTRUCTION);

        dbg!(&not);
        assert_eq!(Register::R_R1, not.destination_register);
        assert_eq!(Register::R_R2, not.source_register);
    }

    #[test]
    fn test_jsr_instruction_initializer() {
        const INSTRUCTION: u16 = 0x400F;
        let jsr = JsrInstruction::new(INSTRUCTION);

        dbg!(&jsr);
        // bit_11 should be 0 for JSR instruction
        assert_eq!(0, jsr.bit_11);
        assert_eq!(15, jsr.offset);
    }
}
