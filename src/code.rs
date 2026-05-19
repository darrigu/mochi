#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opcode {
    OpConstant = 0,
    OpAdd,
    OpSub,
    OpMul,
    OpDiv,
    OpPop,
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
    OpSetFree,
    OpClosure,
    OpArray,
    OpHash,
    OpIndex,
    OpSetIndex,
    OpGetMethod,
}

impl From<u8> for Opcode {
    fn from(val: u8) -> Self {
        if val <= 27 {
            unsafe { std::mem::transmute(val) }
        } else {
            panic!("Unknown Opcode: {}", val)
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
        | Opcode::OpGetMethod => {
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
