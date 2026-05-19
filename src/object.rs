use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Clone, PartialEq)]
#[allow(unpredictable_function_pointer_comparisons)]
pub enum Object {
    Null,
    Number(f64),
    Boolean(bool),
    String(String),
    Hash(Rc<RefCell<HashMap<String, Object>>>),
    CompiledFunction {
        instructions: Vec<u8>,
        constants: Vec<Object>,
        num_locals: usize,
        num_parameters: usize,
    },
    Closure {
        func: Box<Object>,
        free: Vec<Object>,
    },
    Native(fn(Vec<Object>) -> Object),
}

impl Default for Object {
    fn default() -> Self {
        Object::Number(0.0)
    }
}

impl std::fmt::Debug for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Object::Null => write!(f, "null"),
            Object::Number(n) => write!(f, "{}", n),
            Object::Boolean(b) => write!(f, "{}", b),
            Object::String(s) => write!(f, "\"{}\"", s),
            Object::Hash(h) => {
                let map = h.borrow();
                let pairs: Vec<String> =
                    map.iter().map(|(k, v)| format!("{}: {:?}", k, v)).collect();
                write!(f, "{{ {} }}", pairs.join(", "))
            }
            Object::Native(_) => write!(f, "<native fn>"),
            Object::CompiledFunction { .. } => write!(f, "<compiled fn>"),
            Object::Closure { .. } => write!(f, "<closure>"),
        }
    }
}
