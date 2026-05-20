use crate::compiler::Bytecode;
use crate::object::Object;

pub fn serialize(bytecode: &Bytecode) -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"ANKO");

    bytes.extend_from_slice(&(bytecode.instructions.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&bytecode.instructions);

    bytes.extend_from_slice(&(bytecode.constants.len() as u32).to_le_bytes());
    for constant in &bytecode.constants {
        serialize_object(constant, &mut bytes);
    }

    bytes
}

fn serialize_object(obj: &Object, bytes: &mut Vec<u8>) {
    match obj {
        Object::Number(n) => {
            bytes.push(0);
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        Object::String(s) => {
            bytes.push(1);
            bytes.extend_from_slice(&(s.len() as u32).to_le_bytes());
            bytes.extend_from_slice(s.as_bytes());
        }
        Object::Atom(a) => {
            bytes.push(2);
            bytes.extend_from_slice(&(a.len() as u32).to_le_bytes());
            bytes.extend_from_slice(a.as_bytes());
        }
        Object::CompiledFunction {
            instructions,
            num_locals,
            num_parameters,
        } => {
            bytes.push(3);

            bytes.extend_from_slice(&(instructions.len() as u32).to_le_bytes());
            bytes.extend_from_slice(instructions);

            bytes.extend_from_slice(&(*num_locals as u32).to_le_bytes());
            bytes.extend_from_slice(&(*num_parameters as u32).to_le_bytes());
        }
        _ => panic!("Runtime object leaked into constant pool: {:?}", obj),
    }
}

pub fn deserialize(bytes: &[u8]) -> Result<Bytecode, String> {
    if bytes.len() < 4 || &bytes[0..4] != b"ANKO" {
        return Err("Invalid or corrupted .anko file format".to_string());
    }
    let mut offset = 4;

    let inst_len = read_u32(bytes, &mut offset)?;
    let instructions = bytes[offset..offset + inst_len].to_vec();
    offset += inst_len;

    let const_len = read_u32(bytes, &mut offset)?;
    let mut constants = Vec::with_capacity(const_len);
    for _ in 0..const_len {
        constants.push(deserialize_object(bytes, &mut offset)?);
    }

    Ok(Bytecode {
        instructions,
        constants,
    })
}

fn deserialize_object(bytes: &[u8], offset: &mut usize) -> Result<Object, String> {
    let tag = bytes[*offset];
    *offset += 1;

    match tag {
        0 => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[*offset..*offset + 8]);
            *offset += 8;
            Ok(Object::Number(f64::from_le_bytes(buf)))
        }
        1 | 2 => {
            let len = read_u32(bytes, offset)?;
            let s = String::from_utf8(bytes[*offset..*offset + len].to_vec())
                .map_err(|_| "Invalid UTF-8 in string/symbol".to_string())?;
            *offset += len;

            if tag == 1 {
                Ok(Object::String(s))
            } else {
                Ok(Object::Atom(s))
            }
        }
        3 => {
            let inst_len = read_u32(bytes, offset)?;
            let instructions = bytes[*offset..*offset + inst_len].to_vec();
            *offset += inst_len;

            let num_locals = read_u32(bytes, offset)?;
            let num_parameters = read_u32(bytes, offset)?;

            Ok(Object::CompiledFunction {
                instructions,
                num_locals,
                num_parameters,
            })
        }
        _ => Err(format!("Unknown object tag {} in bytecode", tag)),
    }
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<usize, String> {
    if *offset + 4 > bytes.len() {
        return Err("Unexpected end of bytecode".to_string());
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[*offset..*offset + 4]);
    *offset += 4;
    Ok(u32::from_le_bytes(buf) as usize)
}
