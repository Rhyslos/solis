//! A small pool of worker threads for slow jobs (importing models), so the
//! editor never waits (CONTRIBUTING.md, rule 10). Each job is for one
//! asset; when a newer job for the same asset is started, the older one's
//! result is dropped, so a file saved twice quickly is only swapped in
//! once, with its latest content.

use reflect::AssetId;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

type Job<R> = Box<dyn FnOnce() -> R + Send>;

/// Worker threads running jobs, and their results.
pub struct Jobs<R: Send + 'static> {
    sender: Option<Sender<(AssetId, u64, Job<R>)>>,
    results: Receiver<(AssetId, u64, R)>,
    latest: HashMap<AssetId, u64>,
    next: u64,
    running: usize,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl<R: Send + 'static> std::fmt::Debug for Jobs<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Jobs({} running)", self.running)
    }
}

impl<R: Send + 'static> Jobs<R> {
    /// A pool of `threads` workers (at least one).
    pub fn new(threads: usize) -> Self {
        let (sender, queue) = channel::<(AssetId, u64, Job<R>)>();
        let (done, results) = channel();
        let queue = Arc::new(Mutex::new(queue));
        let threads = (0..threads.max(1))
            .map(|_| {
                let queue = Arc::clone(&queue);
                let done = done.clone();
                std::thread::spawn(move || {
                    loop {
                        // Hold the lock only to take the next job.
                        let next = queue.lock().map(|q| q.recv());
                        let Ok(Ok((id, generation, job))) = next else {
                            return;
                        };
                        // A job that panics loses its result, not the
                        // worker.
                        if let Ok(result) =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
                        {
                            let _ = done.send((id, generation, result));
                        }
                    }
                })
            })
            .collect();
        Self {
            sender: Some(sender),
            results,
            latest: HashMap::new(),
            next: 0,
            running: 0,
            threads,
        }
    }

    /// A pool sized for this computer: half its cores, from one to four.
    pub fn for_this_computer() -> Self {
        let cores = std::thread::available_parallelism().map_or(2, usize::from);
        Self::new((cores / 2).clamp(1, 4))
    }

    /// Starts `job` for asset `id`. Any older job for it still running
    /// will have its result dropped.
    pub fn start(&mut self, id: AssetId, job: impl FnOnce() -> R + Send + 'static) {
        self.next += 1;
        self.latest.insert(id, self.next);
        if let Some(sender) = &self.sender
            && sender.send((id, self.next, Box::new(job))).is_ok()
        {
            self.running += 1;
        }
    }

    /// The results finished since last asked, newest jobs only. Never
    /// waits.
    pub fn finished(&mut self) -> Vec<(AssetId, R)> {
        let mut out = Vec::new();
        while let Ok((id, generation, result)) = self.results.try_recv() {
            self.running = self.running.saturating_sub(1);
            if self.latest.get(&id) == Some(&generation) {
                self.latest.remove(&id);
                out.push((id, result));
            }
        }
        out
    }

    /// How many jobs are queued or running.
    pub fn running(&self) -> usize {
        self.running
    }
}

impl<R: Send + 'static> Drop for Jobs<R> {
    fn drop(&mut self) {
        // Closing the queue lets idle workers end; busy ones finish their
        // job first. They aren't waited for, so closing never hangs.
        self.sender = None;
        self.threads.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait<R: Send + 'static>(jobs: &mut Jobs<R>, count: usize) -> Vec<(AssetId, R)> {
        let mut out = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while out.len() < count && Instant::now() < deadline {
            out.extend(jobs.finished());
            std::thread::sleep(Duration::from_millis(5));
        }
        out
    }

    #[test]
    fn jobs_run_on_the_workers_and_come_back() {
        let mut jobs = Jobs::new(2);
        let ids: Vec<AssetId> = (0..5).map(|_| AssetId::random()).collect();
        for (i, id) in ids.iter().enumerate() {
            jobs.start(*id, move || i * 10);
        }
        let mut done = wait(&mut jobs, 5);
        done.sort_by_key(|(_, r)| *r);
        assert_eq!(
            done.iter().map(|(_, r)| *r).collect::<Vec<_>>(),
            [0, 10, 20, 30, 40]
        );
        assert_eq!(jobs.running(), 0);
    }

    #[test]
    fn an_older_result_for_the_same_asset_is_dropped() {
        let mut jobs = Jobs::new(2);
        let id = AssetId::random();
        jobs.start(id, || {
            std::thread::sleep(Duration::from_millis(200));
            "old"
        });
        jobs.start(id, || "new");
        let done = wait(&mut jobs, 1);
        std::thread::sleep(Duration::from_millis(300));
        let late = jobs.finished();
        assert_eq!(done, [(id, "new")]);
        assert!(late.is_empty(), "the slow old one is dropped");
    }

    #[test]
    fn a_job_that_panics_loses_only_its_result() {
        let mut jobs = Jobs::new(1);
        jobs.start(AssetId::random(), || -> u8 { panic!("broken file") });
        let id = AssetId::random();
        jobs.start(id, || 7);
        assert_eq!(wait(&mut jobs, 1), [(id, 7)]);
    }
}
