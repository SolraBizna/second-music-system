use std::future::Future;

/// Something where we can put in `async` functions and have them execute
/// eventually.
///
/// All of our task functions are either of the type "perform some computation
/// and return" or of the type "perform some blocking IO and computation and
/// return". Regular blocking IO is used, not any kind of async IO.
pub trait TaskRuntime: 'static + Send + Sync {
    fn spawn_task(&self, kind: TaskType, task: impl Future<Output=()> + Send + 'static);
}

/// Types of background loading tasks.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Copy, Clone)]
pub enum TaskType {
    /// Background loading of a buffered sound. Should have the lowest
    /// priority.
    BufferLoad,
    /// Background loading of a streamed sound.
    StreamLoad,
    /// Background decoding of a streamed sound. Should have the highest
    /// priority.
    StreamDecode,
}

mod fg;
pub use fg::*;

#[cfg(feature="switchyard")]
mod switchyard;
#[cfg(feature="switchyard")]
pub use self::switchyard::*;

#[cfg(feature="tokio")]
mod tokio;
#[cfg(feature="tokio")]
pub use self::tokio::*;

