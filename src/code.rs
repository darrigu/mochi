#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opcode {
    OpConstant = 0,
    OpAdd,
    OpSub,
    OpMul,
    OpDiv,
    OpPop,
    OpTrue,
    OpFalse,
    OpEqual,
    OpNotEqual,
    OpGreater,
    OpLess,
    OpMinus,
    OpBang,
    OpSetGlobal,
    OpGetGlobal,
    OpJumpNotTruthy,
    OpJump,
    OpCall,
    OpReturnValue,
    OpGetLocal,
    OpSetLocal,
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
            6 => Opcode::OpTrue,
            7 => Opcode::OpFalse,
            8 => Opcode::OpEqual,
            9 => Opcode::OpNotEqual,
            10 => Opcode::OpGreater,
            11 => Opcode::OpLess,
            12 => Opcode::OpMinus,
            13 => Opcode::OpBang,
            14 => Opcode::OpSetGlobal,
            15 => Opcode::OpGetGlobal,
            16 => Opcode::OpJumpNotTruthy,
            17 => Opcode::OpJump,
            18 => Opcode::OpCall,
            19 => Opcode::OpReturnValue,
            20 => Opcode::OpGetLocal,
            21 => Opcode::OpSetLocal,
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
        | Opcode::OpJump => {
            let mut instruction = vec![op as u8];
            let operand = operands[0] as u16;
            instruction.extend_from_slice(&operand.to_be_bytes());
            instruction
        }
        Opcode::OpCall | Opcode::OpGetLocal | Opcode::OpSetLocal => {
            vec![op as u8, operands[0] as u8]
        }
        _ => vec![op as u8],
    }
}
