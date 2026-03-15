pub const CONST: u8 = 1;
pub const TRUE: u8 = 2;
pub const FALSE: u8 = 3;

pub const LOAD_LOCAL: u8 = 4;
pub const LOAD_CAPTURE: u8 = 5;

pub const ADD_INT: u8 = 6;
pub const SUB_INT: u8 = 7;
pub const MUL_INT: u8 = 8;
pub const DIV_INT: u8 = 9;

pub const EQ_INT: u8 = 10;
pub const GT_INT: u8 = 11;
pub const GE_INT: u8 = 12;
pub const LT_INT: u8 = 13;
pub const LE_INT: u8 = 14;

pub const EQ_BOOL: u8 = 15;

pub const JMP: u8 = 16;
pub const JMP_IF_FALSE: u8 = 17;

pub const CLOSURE: u8 = 18;
pub const CALL: u8 = 19;
pub const RET: u8 = 20;

pub fn as_str(opcode: u8) -> &'static str {
    match opcode {
        CONST => "CONST",
        TRUE => "TRUE",
        FALSE => "FALSE",

        LOAD_LOCAL => "LOAD_LOCAL",
        LOAD_CAPTURE => "LOAD_CAPTURE",

        ADD_INT => "ADD_INT",
        SUB_INT => "SUB_INT",
        MUL_INT => "MUL_INT",
        DIV_INT => "DIV_INT",

        EQ_INT => "EQ_INT",
        GT_INT => "GT_INT",
        GE_INT => "GE_INT",
        LT_INT => "LT_INT",
        LE_INT => "LE_INT",

        EQ_BOOL => "EQ_BOOL",

        JMP => "JMP",
        JMP_IF_FALSE => "JMP_IF_FALSE",

        CLOSURE => "CLOSURE",
        CALL => "CALL",
        RET => "RET",
        0 | 21..=u8::MAX => unreachable!(),
    }
}

fn main() {}
