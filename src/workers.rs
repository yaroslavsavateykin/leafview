//! Owned, bounded background tasks. Drop cancels queued work and joins the worker.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc,
};
use std::thread::{self, JoinHandle};

pub struct Worker<I, O> {
    pub input: Option<SyncSender<I>>,
    pub output: Receiver<O>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl<I: Send + 'static, O: Send + 'static> Worker<I, O> {
    pub fn new(f: impl Fn(I, &AtomicBool) -> O + Send + 'static) -> Self {
        let (tx, rx) = mpsc::sync_channel(1);
        let (done, output) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let thread = thread::spawn(move || {
            while let Ok(input) = rx.recv() {
                if stopped.load(Ordering::SeqCst) {
                    break;
                }
                let result = f(input, &stopped);
                if stopped.load(Ordering::SeqCst) || done.try_send(result).is_err() {
                    break;
                }
            }
        });
        Self {
            input: Some(tx),
            output,
            stop,
            thread: Some(thread),
        }
    }
}
impl<I, O> Drop for Worker<I, O> {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.input.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_work_and_owned_shutdown() {
        let worker = Worker::new(|n: u32, stop| {
            while !stop.load(Ordering::SeqCst) {
                thread::sleep(std::time::Duration::from_millis(1));
            }
            n
        });
        worker.input.as_ref().unwrap().send(1).unwrap();
        thread::sleep(std::time::Duration::from_millis(10));
        worker.input.as_ref().unwrap().try_send(2).unwrap();
        assert!(worker.input.as_ref().unwrap().try_send(3).is_err());
        let start = std::time::Instant::now();
        drop(worker);
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }
}
