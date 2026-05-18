#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Number(f64),
    Boolean(bool),
    CompiledFunction {
        instructions: Vec<u8>,
        constants: Vec<Object>,
        num_locals: usize,
        num_parameters: usize,
    },
}
