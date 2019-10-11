use crate::{ctxt_with_diag, log_out};
use pin_project::pin_project;
use std::{fmt, mem};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use std::{borrow::Cow, future::Future, pin::Pin, task::Context, task::Poll, thread::ThreadId};

/// Wraps around `Future` and adds diagnostics to it.
#[pin_project]
#[derive(Clone)]
pub struct DiagnoseFuture<T> {
    /// The inner future doing the actual work.
    #[pin]
    inner: T,
    task_name: Cow<'static, str>,
    task_id: u64,
    first_time_poll: bool,
}

impl<T> DiagnoseFuture<T> {
    pub fn new(inner: T, name: impl Into<Cow<'static, str>>) -> Self {
        // TODO: hack, see doc of elapsed_since_abs_time
        crate::absolute_time::elapsed_since_abs_time(Instant::now());

        DiagnoseFuture {
            inner,
            task_name: name.into(),
            task_id: {
                static NEXT_ID: AtomicU64 = AtomicU64::new(0);
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            },
            first_time_poll: true,
        }
    }
}

impl<T> Future for DiagnoseFuture<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();

        let before = Instant::now();
        let outcome = {
            let waker = ctxt_with_diag::waker_with_diag(cx.waker().clone(), this.task_name.clone(), *this.task_id);
            let mut cx = Context::from_waker(&waker);
            Future::poll(this.inner, &mut cx)
        };
        let after = Instant::now();
        log_out::log_poll(&this.task_name, *this.task_id, before, after, mem::replace(this.first_time_poll, false), outcome.is_ready());
        outcome
    }
}

impl<T> fmt::Debug for DiagnoseFuture<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}
