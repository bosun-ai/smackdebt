//! Delayed terminal feedback; analysis stays on the calling thread.
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(crate) fn run<T>(human_output: bool, work: impl FnOnce() -> T) -> T {
    let _progress = Progress::start(human_output && crate::terminal::interactive());
    work()
}

struct Progress {
    worker: Option<(Sender<()>, JoinHandle<()>)>,
}

impl Progress {
    fn start(enabled: bool) -> Self {
        let worker = enabled.then(|| {
            let (stop, receiver) = mpsc::channel();
            let thread = thread::spawn(move || {
                if receiver.recv_timeout(Duration::from_millis(250))
                    == Err(mpsc::RecvTimeoutError::Timeout)
                {
                    let spinner = cliclack::spinner();
                    spinner.start("Analyzing code…");
                    let _ = receiver.recv();
                    spinner.clear();
                }
            });
            (stop, thread)
        });
        Self { worker }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if let Some((stop, worker)) = self.worker.take() {
            let _ = stop.send(());
            let _ = worker.join();
        }
    }
}
