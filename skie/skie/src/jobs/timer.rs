use std::collections::BinaryHeap;
use std::future::Future;
use std::task::Waker;
use std::time::{Duration, Instant};

use parking::{Parker, Unparker};

use parking_lot::Mutex;
use std::sync::Arc;

// FIXME: can be improved

struct Timeout {
    deadline: Instant,
    waker: Arc<Mutex<Option<Waker>>>,
}

#[derive(Default)]
pub struct TimeoutsManager {
    timeouts: BinaryHeap<Timeout>,
}

impl TimeoutsManager {
    fn insert(&mut self, timeout: Timeout) {
        self.timeouts.push(timeout)
    }

    fn next_expired(&mut self, now: Instant) -> Option<Timeout> {
        if let Some(data) = self.timeouts.peek() {
            if data.deadline > now {
                // not expired
                return None;
            }
            // there is a expired timeout
        } else {
            return None;
        }

        let timeout = self.timeouts.pop().unwrap();
        Some(timeout)
    }
}

pub struct Timer {
    unparker: Unparker,
    _thread: std::thread::JoinHandle<()>,
    timeouts: Arc<Mutex<TimeoutsManager>>,
}

impl std::fmt::Debug for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timer").finish()
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    pub fn new() -> Self {
        let parker = Parker::new();
        let unparker = parker.unparker();

        let timeouts = Arc::new(Mutex::new(TimeoutsManager::default()));
        let _thread = std::thread::spawn({
            let timeouts = timeouts.clone();

            move || loop {
                {
                    let mut lock = timeouts.lock();
                    while let Some(deadline) = lock.next_expired(Instant::now()) {
                        if let Some(waker) = deadline.waker.lock().take() {
                            waker.wake()
                        }
                    }
                }
                parker.park();
            }
        });

        Self {
            unparker,
            _thread,
            timeouts,
        }
    }
    pub fn insert_from_duration(&self, duration: Duration) -> TimeoutFuture {
        self.insert_impl(Instant::now() + duration)
    }

    pub fn insert_from_instant(&self, instant: Instant) -> TimeoutFuture {
        self.insert_impl(instant)
    }

    fn insert_impl(&self, instant: Instant) -> TimeoutFuture {
        let mut lock = self.timeouts.lock();
        let future = TimeoutFuture::from_instant(instant);
        let waker = future.waker.clone();

        let timeout = Timeout {
            deadline: future.deadline,
            waker,
        };

        lock.insert(timeout);
        self.tick();

        future
    }

    pub fn tick(&self) {
        self.unparker.unpark();
    }
}

impl std::cmp::Ord for Timeout {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.deadline.cmp(&self.deadline)
    }
}

impl std::cmp::PartialOrd for Timeout {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::PartialEq for Timeout {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}
impl std::cmp::Eq for Timeout {}

pub struct TimeoutFuture {
    deadline: Instant,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl TimeoutFuture {
    pub fn from_duration(duration: Duration) -> Self {
        Self {
            deadline: Instant::now() + duration,
            waker: Default::default(),
        }
    }

    pub fn from_instant(instant: Instant) -> Self {
        Self {
            deadline: instant,
            waker: Default::default(),
        }
    }
}

impl Future for TimeoutFuture {
    type Output = ();
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            std::task::Poll::Ready(())
        } else {
            *self.waker.lock() = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}

impl Drop for TimeoutFuture {
    fn drop(&mut self) {
        self.waker.lock().take();
    }
}
