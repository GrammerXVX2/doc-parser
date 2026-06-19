use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct BufferPool {
    max_buffers: usize,
    inner: Arc<Mutex<Vec<Vec<f32>>>>,
}

#[derive(Debug, Clone)]
pub struct PooledBuffer {
    pub data: Vec<f32>,
}

impl BufferPool {
    pub fn new(max_buffers: usize) -> Self {
        Self {
            max_buffers: max_buffers.max(1),
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn max_buffers(&self) -> usize {
        self.max_buffers
    }

    pub fn acquire(&self, min_len: usize) -> PooledBuffer {
        let mut selected = None;
        if let Ok(mut pool) = self.inner.lock() {
            if let Some(index) = pool.iter().position(|buf| buf.capacity() >= min_len) {
                selected = Some(pool.swap_remove(index));
            } else if let Some(buf) = pool.pop() {
                selected = Some(buf);
            }
        }

        let mut data = selected.unwrap_or_default();
        if data.capacity() < min_len {
            data.reserve(min_len - data.capacity());
        }
        data.clear();
        data.resize(min_len, 0.0);

        PooledBuffer { data }
    }

    pub fn release(&self, mut buffer: PooledBuffer) {
        buffer.data.clear();
        if let Ok(mut pool) = self.inner.lock() {
            if pool.len() < self.max_buffers {
                pool.push(buffer.data);
            }
        }
    }

    pub fn available_buffers(&self) -> usize {
        self.inner
            .lock()
            .map(|pool| pool.len())
            .unwrap_or_default()
    }
}
