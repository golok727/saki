use async_task::Runnable;
use std::{future::Future, sync::Arc, thread};

#[derive(Debug)]
pub struct Timeout {
    duration: std::time::Duration,
    runnable: Runnable,
}

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
        let (runnable, task) = async_task::spawn(async move {}, {
            let dispatcher = self.dispatcher.clone();
            move |runnable| dispatcher.set_timeout(duration, runnable)
        });
        runnable.schedule();

        Job::Pending(task)
    }

    pub fn spawn_blocking<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Job<T>
    where
        T: Send + 'static,
    {
        self.dispatcher.dispatch_on_thread_pool(future)
    }

    pub fn spawn_local<T>(&self, future: impl Future<Output = T> + 'static) -> Job<T>
    where
        T: 'static,
    {
        self.dispatcher.dispatch_local(future)
    }

    pub fn run_foregound_tasks(&self) {
        for runnable in self.dispatcher.fg_receiver.drain() {
            runnable.run();
        }
    }
}

#[derive(Debug)]
pub struct Dispatcher {
    fg_sender: flume::Sender<Runnable>,

    fg_receiver: flume::Receiver<Runnable>,

    bg_sender: flume::Sender<Runnable>,

    timer: calloop::channel::Sender<Timeout>,

    #[allow(unused)]
    background_threads: Vec<thread::JoinHandle<()>>,
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

        let mut background_threads = (0..thread_count)
            .map(|_| {
                let rx = bg_reciver.clone();
                thread::spawn(move || {
                    for runnable in rx {
                        runnable.run();
                    }
                })
            })
            .collect::<Vec<_>>();

        let (timer_handle, timer) = Self::create_timer();
        background_threads.push(timer_handle);

        Self {
            background_threads,
            bg_sender,
            fg_sender,
            fg_receiver,
            timer,
        }
    }

    pub(crate) fn create_timer() -> (thread::JoinHandle<()>, calloop::channel::Sender<Timeout>) {
        let (sender, channel) = calloop::channel::channel::<Timeout>();
        let handle = std::thread::spawn(|| {
            let mut eventloop: calloop::EventLoop<()> =
                calloop::EventLoop::try_new().expect("error creating timer event_loop");
            let handle = eventloop.handle();
            let timer_handle = eventloop.handle();

            handle
                .insert_source(channel, move |e, _, _| {
                    if let calloop::channel::Event::Msg(timeout) = e {
                        let mut runnable = Some(timeout.runnable);

                        timer_handle
                            .insert_source(
                                calloop::timer::Timer::from_duration(timeout.duration),
                                move |_, _, _| {
                                    if let Some(runnable) = runnable.take() {
                                        runnable.run();
                                    }
                                    calloop::timer::TimeoutAction::Drop
                                },
                            )
                            .expect("unable to start timer");
                    };
                })
                .expect("failed to start timer");

            if let Err(err) = eventloop.run(None, &mut (), |_| {}) {
                eprintln!("{}", err);
            }
        });

        (handle, sender)
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

    pub fn dispatch_local<T>(&self, future: impl Future<Output = T> + 'static) -> Job<T>
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

    fn set_timeout(&self, duration: std::time::Duration, runnable: Runnable) {
        self.timer.send(Timeout { duration, runnable }).ok();
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
