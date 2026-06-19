#[derive(Debug, Clone)]
pub struct OnnxTensor {
    pub name: String,
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

impl OnnxTensor {
    pub fn new(name: impl Into<String>, shape: Vec<usize>, data: Vec<f32>) -> Self {
        Self {
            name: name.into(),
            shape,
            data,
        }
    }
}
