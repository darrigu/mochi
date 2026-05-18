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
    OpGetFree,
    OpClosure,
}

impl From<u8> for Opcode {
    fn from(val: u8) -> Self {
        match val {
            0..=23 => unsafe { std::mem::transmute(val) },
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
        Opcode::OpCall
        | Opcode::OpGetLocal
        | Opcode::OpSetLocal
        | Opcode::OpGetFree
        | Opcode::OpClosure => {
            let mut instruction = vec![op as u8];
            let operand = operands[0] as u16;
            if op == Opcode::OpClosure {
                instruction.extend_from_slice(&operand.to_be_bytes());
                instruction.push(operands[1] as u8);
            } else {
                instruction.push(operands[0] as u8);
            }
            instruction
        }
        _ => vec![op as u8],
    }
}
