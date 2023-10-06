//! This module is used to asynchronously answer simple queries across thread
//! boundaries. A query is a question that can only be answered by one thread,
//! and only be answered to one thread. In essence, this is a one-shot
//! single-producer-single-consumer channel.

use std::{
    cell::UnsafeCell,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    future::Future,
    sync::{Arc, atomic::{AtomicBool,Ordering}},
    task::Poll,
};

use futures::task::AtomicWaker;

struct Inner<T: Send + Sized> {
    ready: AtomicBool,
    waker: AtomicWaker,
    response: UnsafeCell<Option<T>>,
}

unsafe impl<T: Send + Sync> Sync for Inner<T> {}

/// The response to a query.
/// [See module-level documentation for more info.](index.html)
pub struct Response<T: Send + Sized>(Arc<Inner<T>>);
/// The means with which to respond to a query.
/// [See module-level documentation for more info.](index.html)
pub struct Responder<T: Send + Sized>(Arc<Inner<T>>);

impl<T: Send + Sized> Debug for Responder<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "query::Responder")
    }
}

// specialization is unstable (#31844) so we can't have a specialized version
// for T: Debug
impl<T: Send + Sized> Debug for Response<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "query::Response(")?;
        if self.0.ready.load(Ordering::Relaxed) {
            write!(f, "ready")?;
        } else {
            write!(f, "pending...")?;
        }
        write!(f, ")")
    }
}

impl<T: Send + Sized> Drop for Responder<T> {
    fn drop(&mut self) {
        self.0.ready.store(true, Ordering::Release)
    }
}

/// Initiates a query.
/// [See module-level documentation for more info.](index.html)
pub fn make<T: Send + Sized>() -> (Responder<T>, Response<T>) {
    let inner = Arc::new(Inner{
        ready: AtomicBool::new(false),
        waker: AtomicWaker::new(),
        response: UnsafeCell::new(None),
    });
    (Responder(inner.clone()), Response(inner))
}

impl<T: Send + Sized> Responder<T> {
    pub fn respond(self, value: T) {
        unsafe { self.0.response.get().write(Some(value)) };
        self.0.ready.store(true, Ordering::Release);
        self.0.waker.wake();
    }
}

impl<T: Send + Sized> Response<T> {
    /// Checks if the response is ready yet. If this function returns true,
    /// `get` and `take` will only return `Some` or panic, never return `None`.
    pub fn poll(&self) -> bool {
        self.0.ready.load(Ordering::Relaxed)
    }
    /// If the response is ready, returns `Some(&response)`. If it's not ready
    /// yet, returns `None`. If the responder was dropped without sending a
    /// response, or the response has arrived but was already taken, panics.
    pub fn get(&mut self) -> Option<&T> {
        match self.try_get() {
            Ok(x) => Some(x),
            Err(TryGetError::NotReady) => None,
            Err(e) => panic!("{e}"),
        }
    }
    /// If the response is ready, returns `Some(response)`. If it's not ready
    /// yet, returns `None`. If the responder was dropped without sending a
    /// response, or the response has arrived but was already taken, panics.
    pub fn take(&mut self) -> Option<T> {
        match self.try_take() {
            Ok(x) => Some(x),
            Err(TryGetError::NotReady) => None,
            Err(e) => panic!("{e}"),
        }
    }
    /// If the response is ready, returns `Ok(&response)`. In any other
    /// circumstances, returns a `TryGetError` variant.
    pub fn try_get(&mut self) -> Result<&T, TryGetError> {
        if self.0.ready.load(Ordering::Acquire) {
            match unsafe { self.0.response.get().as_ref().unwrap().as_ref() } {
                Some(x) => Ok(x),
                None => Err(TryGetError::MissingResponse),
            }
        } else {
            Err(TryGetError::NotReady)
        }
    }
    /// If the response is ready, returns `Ok(response)`. In any other
    /// circumstances, returns a `TryTakeError` variant. It is a logic error to
    /// do anything with this `Response` after a successful `take` or
    /// `try_take`.
    pub fn try_take(&mut self) -> Result<T, TryTakeError> {
        if self.0.ready.load(Ordering::Acquire) {
            match unsafe { self.0.response.get().as_mut().unwrap().take() } {
                Some(x) => Ok(x),
                None => Err(TryTakeError::MissingResponse),
            }
        } else {
            Err(TryTakeError::NotReady)
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TryGetError {
    NotReady,
    MissingResponse,
}
pub type TryTakeError = TryGetError;

impl Display for TryGetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            TryGetError::NotReady => write!(f, "response not ready yet"),
            TryGetError::MissingResponse => write!(f, "response ready but missing (responder dropped unspent, or .take()/.try_take()/.await performed more than once)"),
        }
    }
}

impl<T: Send + Sized> Future for Response<T> {
    type Output = T;
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<T> {
        match self.take() {
            Some(x) => Poll::Ready(x),
            None => {
                self.0.waker.register(cx.waker());
                match self.take() {
                    Some(x) => Poll::Ready(x),
                    None => Poll::Pending,
                }
            }
        }
    }
}
