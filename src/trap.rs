use std::io::{Read, Write, stdin, stdout};

use crate::{Register, Svm};

pub enum TrapCode {
    GETC = 0x20,
    OUT,   // output a character from keyboard input
    PUTS,  // output a word
    IN,    // get character from the keyboard and echoed in terminal
    PUTSP, // output a byte
    HALT,  // halt a program
}

impl TrapCode {
    fn new(code: u8) -> Self {
        match code {
            0x20 => TrapCode::GETC,
            0x21 => TrapCode::OUT,
            0x22 => TrapCode::PUTS,
            0x23 => TrapCode::IN,
            0x24 => TrapCode::PUTSP,
            0x25 => TrapCode::HALT,
            _ => panic!("unimplemented trap code: {code}"),
        }
    }
}
#[allow(static_mut_refs)]
pub fn trap(svm: &mut Svm) -> TrapCode {
    let instr = svm.current_instr();
    svm.reg[Register::R_R7.as_usize()] = svm.reg[Register::R_PC.as_usize()];
    let trap_code = TrapCode::new((instr & 0xFF) as u8);

    match &trap_code {
        TrapCode::GETC => {
            let byte = stdin()
                .bytes()
                .next()
                .and_then(|r| r.ok())
                .expect("unable to read character");

            svm.reg[Register::R_R0.as_usize()] = byte as u16;
            svm.update_flags(Register::R_R0);
        }
        TrapCode::OUT => {
            let c = svm.reg[Register::R_R0.as_usize()] & 0xFF;
            print!("{}", c as u8 as char);
            stdout().flush().unwrap();
        }
        TrapCode::PUTS => {
            let mut addr = svm.reg[Register::R_R0.as_usize()];
            loop {
                let word = svm.mem_read(addr as usize);
                if word == 0 {
                    break;
                }
                let word = (word & 0xFF) as u8;

                print!("{}", word as char);
                addr += 1;
            }

            stdout().flush().unwrap();
        }
        TrapCode::IN => {
            print!("Enter a character: ");
            stdout().flush().unwrap();
            let byte = stdin()
                .bytes()
                .next()
                .and_then(|r| r.ok())
                .expect("unable to read character");
            print!("{}", byte as char);
            stdout().flush().unwrap();

            svm.reg[Register::R_R0.as_usize()] = byte as u16;
            svm.update_flags(Register::R_R0);
        }
        TrapCode::PUTSP => {
            let mut addr = svm.reg[Register::R_R0.as_usize()];
            loop {
                let word = svm.mem_read(addr as usize);
                if word == 0 {
                    break;
                }

                let char1 = (word & 0xFF) as u8;
                print!("{}", char1 as char);

                let char2 = (word >> 8) as u8;
                if char2 != 0 {
                    print!("{}", char2 as char);
                }

                addr += 1;
            }
            stdout().flush().unwrap();
        }
        TrapCode::HALT => {
            print!("BYE");
            stdout().flush().unwrap();
        }
    }

    trap_code
}
