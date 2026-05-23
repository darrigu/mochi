#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    OpConstant = 0,
    OpAdd = 1,
    OpSub = 2,
    OpMul = 3,
    OpDiv = 4,
    OpPop = 5,
    OpEqual = 6,
    OpNotEqual = 7,
    OpGreater = 8,
    OpLess = 9,
    OpMinus = 10,
    OpBang = 11,
    OpSetGlobal = 12,
    OpGetGlobal = 13,
    OpJumpNotTruthy = 14,
    OpJump = 15,
    OpCall = 16,
    OpReturnValue = 17,
    OpGetLocal = 18,
    OpSetLocal = 19,
    OpGetFree = 20,
    OpSetFree = 21,
    OpClosure = 22,
    OpArray = 23,
    OpHash = 24,
    OpIndex = 25,
    OpSetIndex = 26,
    OpGetMethod = 27,
    OpArrayLen = 28,
    OpHashKeys = 29,
    OpTuple = 30,
    OpTupleLen = 31,
}

impl From<u8> for Opcode {
    fn from(val: u8) -> Self {
        match val {
            0 => Opcode::OpConstant,
            1 => Opcode::OpAdd,
            2 => Opcode::OpSub,
            3 => Opcode::OpMul,
            4 => Opcode::OpDiv,
            5 => Opcode::OpPop,
            6 => Opcode::OpEqual,
            7 => Opcode::OpNotEqual,
            8 => Opcode::OpGreater,
            9 => Opcode::OpLess,
            10 => Opcode::OpMinus,
            11 => Opcode::OpBang,
            12 => Opcode::OpSetGlobal,
            13 => Opcode::OpGetGlobal,
            14 => Opcode::OpJumpNotTruthy,
            15 => Opcode::OpJump,
            16 => Opcode::OpCall,
            17 => Opcode::OpReturnValue,
            18 => Opcode::OpGetLocal,
            19 => Opcode::OpSetLocal,
            20 => Opcode::OpGetFree,
            21 => Opcode::OpSetFree,
            22 => Opcode::OpClosure,
            23 => Opcode::OpArray,
            24 => Opcode::OpHash,
            25 => Opcode::OpIndex,
            26 => Opcode::OpSetIndex,
            27 => Opcode::OpGetMethod,
            28 => Opcode::OpArrayLen,
            29 => Opcode::OpHashKeys,
            30 => Opcode::OpTuple,
            31 => Opcode::OpTupleLen,
            _ => panic!("Unknown Opcode: {}", val),
        }
    }
}

pub fn make(op: Opcode, operands: &[usize]) -> Vec<u8> {
    match op {
        Opcode::OpConstant
        | Opcode::OpSetGlobal
        | Opcode::OpGetGlobal
        | Opcode::OpJumpNotTruthy
        | Opcode::OpJump
        | Opcode::OpArray
        | Opcode::OpHash
        | Opcode::OpGetMethod
        | Opcode::OpTuple => {
            let mut instruction = Vec::with_capacity(3);
            instruction.push(op as u8);
            instruction.extend_from_slice(&(operands[0] as u16).to_be_bytes());
            instruction
        }

        Opcode::OpCall
        | Opcode::OpGetLocal
        | Opcode::OpSetLocal
        | Opcode::OpGetFree
        | Opcode::OpSetFree => {
            vec![op as u8, operands[0] as u8]
        }

        Opcode::OpClosure => {
            let mut instruction = Vec::with_capacity(4);
            instruction.push(op as u8);
            instruction.extend_from_slice(&(operands[0] as u16).to_be_bytes());
            instruction.push(operands[1] as u8);
            instruction
        }

        _ => vec![op as u8],
    }
}
