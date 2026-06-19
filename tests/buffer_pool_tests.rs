use document_parser::performance::{BufferPool, PooledBuffer};

#[test]
fn acquire_creates_buffer() {
    let pool = BufferPool::new(2);
    let buffer = pool.acquire(16);
    assert_eq!(buffer.data.len(), 16);
}

#[test]
fn release_reuses_buffer() {
    let pool = BufferPool::new(2);
    let mut buffer = pool.acquire(8);
    buffer.data[0] = 42.0;
    pool.release(buffer);

    assert_eq!(pool.available_buffers(), 1);
    let reused = pool.acquire(8);
    assert_eq!(reused.data.len(), 8);
    assert_eq!(pool.available_buffers(), 0);
}

#[test]
fn min_len_respected() {
    let pool = BufferPool::new(2);
    pool.release(PooledBuffer { data: vec![0.0; 4] });

    let big = pool.acquire(32);
    assert_eq!(big.data.len(), 32);
}

#[test]
fn max_buffers_respected() {
    let pool = BufferPool::new(1);
    pool.release(PooledBuffer { data: vec![0.0; 4] });
    pool.release(PooledBuffer { data: vec![0.0; 8] });

    assert_eq!(pool.available_buffers(), 1);
}
