use std::{cell::RefCell, rc::Rc};

use indexmap::IndexMap;

#[derive(Clone, PartialEq)]
#[allow(unpredictable_function_pointer_comparisons)]
pub enum Object {
    Number(f64),
    String(String),
    Atom(String),
    Array(Rc<RefCell<Vec<Object>>>),
    Hash(Rc<RefCell<IndexMap<String, Object>>>),
    CompiledFunction {
        instructions: Vec<u8>,
        num_locals: usize,
        num_parameters: usize,
    },
    Closure {
        func: Box<Object>,
        free: Vec<Object>,
    },
    Native(fn(Vec<Object>) -> Object),
    Tuple(Vec<Object>),
}

impl Default for Object {
    fn default() -> Self {
        Object::Number(0.0)
    }
}

impl std::fmt::Debug for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Object::Number(n) => write!(f, "{}", n),
            Object::String(s) => write!(f, "\"{}\"", s),
            Object::Atom(s) => write!(f, ":{}", s),
            Object::Array(arr) => {
                let list = arr.borrow();
                let items: Vec<String> = list.iter().map(|item| format!("{:?}", item)).collect();
                write!(f, "[{}]", items.join(", "))
            }
            Object::Hash(h) => {
                let map = h.borrow();
                let pairs: Vec<String> =
                    map.iter().map(|(k, v)| format!("{}: {:?}", k, v)).collect();
                write!(f, "{{ {} }}", pairs.join(", "))
            }
            Object::Native(_) => write!(f, "<native fn>"),
            Object::CompiledFunction { .. } => write!(f, "<compiled fn>"),
            Object::Closure { .. } => write!(f, "<closure>"),
            Object::Tuple(elements) => {
                if elements.len() == 1 {
                    write!(f, "({:?},)", elements[0])
                } else {
                    let items: Vec<String> =
                        elements.iter().map(|item| format!("{:?}", item)).collect();
                    write!(f, "({})", items.join(", "))
                }
            }
        }
    }
}
