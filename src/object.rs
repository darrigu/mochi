#[derive(Clone, PartialEq)]
#[allow(unpredictable_function_pointer_comparisons)]
pub enum Object {
    Number(f64),
    Boolean(bool),
    String(String),
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
            Object::Number(n) => write!(f, "{}", n),
            Object::Boolean(b) => write!(f, "{}", b),
            Object::String(s) => write!(f, "\"{}\"", s),
            Object::Native(_) => write!(f, "<native fn>"),
            Object::CompiledFunction { .. } => write!(f, "<compiled fn>"),
            Object::Closure { .. } => write!(f, "<closure>"),
        }
    }
}
