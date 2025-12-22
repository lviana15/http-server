pub struct ThreadPool {
    workers: Vec<u32>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        Self {
            workers: Vec::with_capacity(size),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
    }
}
