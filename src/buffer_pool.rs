use std::{
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc},
};
use parking_lot::Mutex;

pub trait BufferFactory<T, P> {
    fn create(&mut self, width: u32, height: u32, stride: usize, format: &P) -> FrameBuffer<T, P>;
    fn free(&mut self, buffer: FrameBuffer<T, P>);
}

#[derive(Clone)]
pub struct FrameBuffer<T, P> {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub format: P,
    pub inner: T,
}

impl<T, P: fmt::Debug> fmt::Debug for FrameBuffer<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameBuffer")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("stride", &self.stride)
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}

/// Key identifying a bucket of compatible frame buffers.
#[derive(Clone, Debug)]
struct BufKey<P> {
    width: u32,
    height: u32,
    stride: usize,
    format: P,
}

impl<P: PartialEq> PartialEq for BufKey<P> {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.stride == other.stride
            && self.format == other.format
    }
}

impl<P: Eq> Eq for BufKey<P> {}

impl<P: Hash> Hash for BufKey<P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.width.hash(state);
        self.height.hash(state);
        self.stride.hash(state);
        self.format.hash(state);
    }
}

/// The inner shared state of the pool.
struct PoolInner<T, P, F>
where
    P: Eq + Hash + Clone + Send + Sync + 'static,
    F: BufferFactory<T, P>,
{
    capacity_per_key: usize,
    factory: Mutex<F>,
    // Buckets keyed by (w,h,stride,format). Each holds returned/available buffers.
    buckets: Mutex<HashMap<BufKey<P>, Vec<FrameBuffer<T, P>>>>,
}

/// Public handle to the pool.
#[derive(Clone)]
pub struct BufferPool<T, P, F>
where
    P: Eq + Hash + Clone + Send + Sync + 'static,
    F: BufferFactory<T, P>,
{
    inner: Arc<PoolInner<T, P, F>>,
}

impl<T, P, F> BufferPool<T, P, F>
where
    P: Eq + Hash + Clone + Send + Sync + 'static,
    F: BufferFactory<T, P>,
{
    /// Create a pool with a per-key capacity and a factory that constructs new buffers.
    ///
    /// `capacity_per_key` is the maximum number of **idle** buffers retained per (w,h,stride,format).
    /// When a returned buffer would exceed this, it is dropped instead of being kept.
    pub fn new(capacity_per_key: usize, factory: F) -> Self {
        Self {
            inner: Arc::new(PoolInner {
                capacity_per_key,
                factory: Mutex::new(factory),
                buckets: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Get a frame buffer matching the (width, height, stride, format) parameters.
    /// Reuses a buffer from the pool if available, otherwise creates a new one via the factory.
    pub fn get(&self, width: u32, height: u32, stride: usize, format: P) -> PooledFrame<T, P, F> {
        let key = BufKey {
            width,
            height,
            stride,
            format: format.clone(),
        };

        // Try to grab a buffer from the bucket.
        let maybe_buf = {
            let mut buckets = self.inner.buckets.lock();
            if let Some(vec) = buckets.get_mut(&key) {
                vec.pop()
            } else {
                None
            }
        };

        let buf = match maybe_buf {
            Some(buf) => buf,
            None => self.inner.factory.lock().create(width, height, stride, &format),
        };

        PooledFrame {
            pool: Some(self.inner.clone()),
            key,
            buf: Some(buf),
            // Whether this pooled frame should return to the pool on drop.
            return_on_drop: true,
        }
    }

    /*/// Manually release a buffer back into the pool. (Usually not needed; happens on Drop.)
    fn release_internal(&self, key: BufKey<P>, mut buf: FrameBuffer<T, P>) {
        // If the buffer was mutated externally to a different shape/format (shouldn't happen),
        // you could validate here. We'll trust the caller, as `key` comes from us.
        let mut buckets = self.inner.buckets.lock();
        let entry = buckets.entry(key.clone()).or_default();
        if entry.len() < self.inner.capacity_per_key {
            // Optional: shrink to fit to avoid holding onto huge Vecs:
            // buf.data.shrink_to_fit();
            entry.push(buf);
        } else {
            self.inner.factory.lock().free(buf);
        }
    }*/
}
impl<T, P, F> Drop for PoolInner<T, P, F>
where
    P: Eq + Hash + Clone + Send + Sync + 'static,
    F: BufferFactory<T, P>,
{
    fn drop(&mut self) {
        // Free all buffers in all buckets.
        let mut factory = self.factory.lock();
        let mut buckets = self.buckets.lock();
        for (_key, vec) in buckets.drain() {
            for buf in vec {
                factory.free(buf);
            }
        }
    }
}

/// A smart handle that returns its buffer to the pool on drop.
pub struct PooledFrame<T, P, F>
where
    P: Eq + Hash + Clone + Send + Sync + 'static,
    F: BufferFactory<T, P>,
{
    pool: Option<Arc<PoolInner<T, P, F>>>,
    key: BufKey<P>,
    buf: Option<FrameBuffer<T, P>>,
    return_on_drop: bool,
}

impl<T, P, F> fmt::Debug for PooledFrame<T, P, F>
where
    P: Eq + Hash + Clone + Send + Sync + fmt::Debug + 'static,
    F: BufferFactory<T, P>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PooledFrame")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl<T, P, F> PooledFrame<T, P, F>
where
    P: Eq + Hash + Clone + Send + Sync + 'static,
    F: BufferFactory<T, P>,
{
    /// Access the buffer.
    pub fn buffer(&self) -> &FrameBuffer<T, P> {
        self.buf.as_ref().expect("buffer already taken")
    }

    /// Mutable access to the buffer.
    pub fn buffer_mut(&mut self) -> &mut FrameBuffer<T, P> {
        self.buf.as_mut().expect("buffer already taken")
    }

    /// Consume and prevent returning to the pool (the buffer is yours to keep).
    pub fn into_inner(mut self) -> FrameBuffer<T, P> {
        self.return_on_drop = false;
        self.buf.take().expect("buffer already taken")
    }

    /// Explicitly release early. After this, the handle is empty and Drop is a no-op.
    pub fn release(mut self) {
        if let (Some(pool), Some(buf)) = (self.pool.take(), self.buf.take()) {
            // Reinsert under lock, observing capacity.
            let mut buckets = pool.buckets.lock();
            let entry = buckets.entry(self.key.clone()).or_default();
            if entry.len() < pool.capacity_per_key {
                entry.push(buf);
            } else {
                pool.factory.lock().free(buf);
            }
        }
        self.return_on_drop = false;
    }
}

impl<T, P, F> Drop for PooledFrame<T, P, F>
where
    P: Eq + Hash + Clone + Send + Sync + 'static,
    F: BufferFactory<T, P>,
{
    fn drop(&mut self) {
        if self.return_on_drop {
            if let (Some(pool), Some(buf)) = (self.pool.take(), self.buf.take()) {
                let mut buckets = pool.buckets.lock();
                let entry = buckets.entry(self.key.clone()).or_default();
                if entry.len() < pool.capacity_per_key {
                    entry.push(buf);
                } else {
                    pool.factory.lock().free(buf);
                }
            }
        }
    }
}
