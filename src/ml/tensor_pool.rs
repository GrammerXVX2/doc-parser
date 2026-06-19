use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
pub struct TensorBuffer {
    pub data: Vec<f32>,
}

pub trait TensorAllocator {
    fn allocate_f32(&self, len: usize) -> TensorBuffer;
}

#[derive(Debug, Clone)]
pub struct TensorPool {
    max_buffers: usize,
    buffers: Arc<Mutex<Vec<Vec<f32>>>>,
}

impl TensorPool {
    pub fn new(max_buffers: usize) -> Self {
        Self {
            max_buffers: max_buffers.max(1),
            buffers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn release(&self, mut buffer: TensorBuffer) {
        if let Ok(mut guard) = self.buffers.lock() {
            if guard.len() >= self.max_buffers {
                return;
            }
            buffer.data.clear();
            guard.push(buffer.data);
        }
    }

    pub fn available_buffers(&self) -> usize {
        self.buffers.lock().map(|guard| guard.len()).unwrap_or(0)
    }
}

impl TensorAllocator for TensorPool {
    fn allocate_f32(&self, len: usize) -> TensorBuffer {
        if let Ok(mut guard) = self.buffers.lock() {
            if let Some(index) = guard.iter().position(|buf| buf.capacity() >= len) {
                let mut data = guard.swap_remove(index);
                data.resize(len, 0.0);
                return TensorBuffer { data };
            }
        }

        TensorBuffer {
            data: vec![0.0; len],
        }
    }
}
