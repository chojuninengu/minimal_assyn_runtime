use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    thread,
    time::{Duration, Instant},
};

pub struct MiniRuntime {
    tasks: VecDeque<Task>,
    timers: VecDeque<Timer>,
}

struct Task {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    waker: Option<Waker>,
}

struct Timer {
    deadline: Instant,
    waker: Waker,
}

impl MiniRuntime {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            timers: VecDeque::new(),
        }
    }

    pub fn block_on<F: Future>(&mut self, future: F) -> F::Output {
        let mut future = Box::pin(future);
        let waker = self.create_waker();
        let mut cx = Context::from_waker(&waker);

        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }

            self.process_timers();
            self.process_tasks();

            if self.tasks.is_empty() && self.timers.is_empty() {
                thread::yield_now();
            }
        }
    }

    fn create_waker(&self) -> Waker {
        static VTABLE: RawWakerVTable = RawWakerVTable::new(
            |_| RawWaker::new(std::ptr::null(), &VTABLE),
            |_| {},
            |_| {},
            |_| {},
        );
        unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
    }

    fn process_timers(&mut self) {
        let now = Instant::now();
        while let Some(timer) = self.timers.front() {
            if timer.deadline <= now {
                let timer = self.timers.pop_front().unwrap();
                timer.waker.wake();
            } else {
                break;
            }
        }
    }

    fn process_tasks(&mut self) {
        let mut tasks = std::mem::take(&mut self.tasks);
        while let Some(mut task) = tasks.pop_front() {
            let waker = task.waker.take().unwrap();
            let mut cx = Context::from_waker(&waker);

            if let Poll::Pending = task.future.as_mut().poll(&mut cx) {
                task.waker = Some(waker);
                self.tasks.push_back(task);
            }
        }
    }
}

pub async fn sleep(duration: Duration) {
    struct Sleep {
        deadline: Instant,
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if Instant::now() >= self.deadline {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }

    Sleep {
        deadline: Instant::now() + duration,
    }
    .await
}

pub async fn yield_now() {
    struct YieldNow {
        yielded: bool,
    }

    impl Future for YieldNow {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.yielded {
                Poll::Ready(())
            } else {
                self.yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    YieldNow { yielded: false }.await
}

#[macro_export]
macro_rules! mini_rt {
    ($($t:tt)*) => {
        fn main() {
            let mut rt = $crate::MiniRuntime::new();
            rt.block_on(async { $($t)* });
        }
    };
}
