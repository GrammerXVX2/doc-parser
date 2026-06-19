#[derive(Debug, Clone)]
pub struct DynamicBatcher<T, R> {
    pub max_batch_size: usize,
    pub max_wait_ms: u64,
    _phantom_t: std::marker::PhantomData<T>,
    _phantom_r: std::marker::PhantomData<R>,
}

impl<T, R> DynamicBatcher<T, R> {
    pub fn new(max_batch_size: usize, max_wait_ms: u64) -> Self {
        Self {
            max_batch_size: max_batch_size.max(1),
            max_wait_ms,
            _phantom_t: std::marker::PhantomData,
            _phantom_r: std::marker::PhantomData,
        }
    }
}

pub fn chunk_batches<T>(items: Vec<T>, max_batch_size: usize) -> Vec<Vec<T>> {
    if items.is_empty() {
        return vec![];
    }

    let batch = max_batch_size.max(1);
    let mut out = Vec::new();
    let mut current = Vec::with_capacity(batch);

    for item in items {
        current.push(item);
        if current.len() >= batch {
            out.push(std::mem::take(&mut current));
            current = Vec::with_capacity(batch);
        }
    }

    if !current.is_empty() {
        out.push(current);
    }

    out
}
