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
    OpMinus,
    OpBang,
    OpSetGlobal,
    OpGetGlobal,
    OpJumpNotTruthy,
    OpJump,
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
            10 => Opcode::OpMinus,
            11 => Opcode::OpBang,
            12 => Opcode::OpSetGlobal,
            13 => Opcode::OpGetGlobal,
            14 => Opcode::OpJumpNotTruthy,
            15 => Opcode::OpJump,
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
        Opcode::OpAdd
        | Opcode::OpSub
        | Opcode::OpMul
        | Opcode::OpDiv
        | Opcode::OpPop
        | Opcode::OpTrue
        | Opcode::OpFalse
        | Opcode::OpEqual
        | Opcode::OpNotEqual
        | Opcode::OpMinus
        | Opcode::OpBang => {
            vec![op as u8]
        }
    }
}
