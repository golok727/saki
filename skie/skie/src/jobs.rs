use async_task::Runnable;
use std::{future::Future, sync::Arc, thread, time::Instant};
use timer::Timer;

pub mod timer;

#[derive(Debug, Clone)]
pub struct Jobs {
    dispatcher: Arc<Dispatcher>,
}

impl Jobs {
    pub fn new(max_threads: Option<usize>) -> Self {
        Self {
            dispatcher: Arc::new(Dispatcher::new(max_threads)),
        }
    }

    pub fn timer(&self, duration: std::time::Duration) -> Job<()> {
        let timeout = self.dispatcher.sleep(duration);
        self.spawn(timeout)
    }

    pub fn spawn_blocking<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Job<T>
    where
        T: Send + 'static,
    {
        self.dispatcher.dispatch_on_thread_pool(future)
    }

    pub fn spawn<T>(&self, future: impl Future<Output = T> + 'static) -> Job<T>
    where
        T: 'static,
    {
        self.dispatcher.dispatch_on_main(future)
    }

    pub fn run_foregound_tasks(&self) {
        self.dispatcher.run_foregound_tasks();
    }
}

// TODO: Move to trait
#[derive(Debug)]
pub struct Dispatcher {
    fg_sender: flume::Sender<Runnable>,

    fg_receiver: flume::Receiver<Runnable>,

    bg_sender: flume::Sender<Runnable>,

    timer: Timer,

    _background_threads: Vec<thread::JoinHandle<()>>,
}

impl Dispatcher {
    pub fn new(max_threads: Option<usize>) -> Self {
        let (bg_sender, bg_reciver) = flume::unbounded::<Runnable>();

        let (fg_sender, fg_receiver) = flume::unbounded::<Runnable>();

        let avail_threads = thread::available_parallelism()
            .map(|v| v.get())
            .unwrap_or(1);

        let thread_count = max_threads
            .unwrap_or(avail_threads)
            .min(avail_threads)
            .max(1);

        log::info!(
            "Creating dispatcher with {} background threads",
            thread_count
        );

        let mut _background_threads = (0..thread_count)
            .map(|_| {
                let rx = bg_reciver.clone();
                thread::spawn(move || {
                    for runnable in rx {
                        let now = Instant::now();
                        runnable.run();
                        log::trace!(
                            "Background thread ran task took: {}ms",
                            Instant::now().saturating_duration_since(now).as_millis()
                        );
                    }
                })
            })
            .collect::<Vec<_>>();

        let timer = Timer::new();
        Self {
            _background_threads,
            bg_sender,
            fg_sender,
            fg_receiver,
            timer,
        }
    }

    pub fn run_foregound_tasks(&self) {
        self.timer.tick();
        for runnable in self.fg_receiver.drain() {
            runnable.run();
        }
    }

    pub fn dispatch_on_thread_pool<T>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Job<T>
    where
        T: Send + 'static,
    {
        let future_pin = Box::pin(future);
        let sender = self.bg_sender.clone();

        let (runnable, task) =
            async_task::spawn(future_pin, move |runnable| sender.send(runnable).unwrap());

        runnable.schedule();

        Job::Pending(task)
    }

    pub fn dispatch_on_main<T>(&self, future: impl Future<Output = T> + 'static) -> Job<T>
    where
        T: 'static,
    {
        let future_pin = Box::pin(future);
        let sender = self.fg_sender.clone();

        let (runnable, task) =
            async_task::spawn_local(future_pin, move |runnable| sender.send(runnable).unwrap());

        runnable.schedule();

        Job::Pending(task)
    }

    fn sleep(&self, duration: std::time::Duration) -> impl Future<Output = ()> {
        self.timer.insert_from_duration(duration)
    }
}

pub enum Job<T> {
    Ready(Option<T>),
    Pending(async_task::Task<T>),
}

impl<T> Job<T> {
    pub fn detach(self) {
        match self {
            Job::Ready(_) => {}
            Job::Pending(task) => task.detach(),
        };
    }
}

impl<T> Future for Job<T> {
    type Output = T;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match unsafe { self.get_unchecked_mut() } {
            Job::Ready(item) => std::task::Poll::Ready(item.take().unwrap()),

            Job::Pending(task) => {
                let mut pinned = std::pin::pin!(task);
                pinned.as_mut().poll(cx)
            }
        }
    }
}
