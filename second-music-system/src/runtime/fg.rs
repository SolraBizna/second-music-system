use std::{task::{Context, Waker, RawWaker, RawWakerVTable}, pin::pin};

use super::*;

/// A very unsophisticated `TaskRuntime` that just immediately and
/// synchronously executes any task you send it. Used to implement "foreground
/// loading", for example for offline rendering of replays or GEFMVs.
pub struct ForegroundTaskRuntime;

static RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |_| RawWaker::new(&(), &RAW_WAKER_VTABLE),
    |_| unreachable!(),
    |_| unreachable!(),
    |_| {},
);

impl TaskRuntime for ForegroundTaskRuntime {
    fn spawn_task(&self, _kind: TaskType, task: impl Future<Output=()> + Send + 'static) {
        let waker = unsafe { Waker::from_raw(RawWaker::new(&(), &RAW_WAKER_VTABLE)) };
        let mut context = Context::from_waker(&waker);
        let mut pin = pin!(task);
        while Future::poll(pin.as_mut(), &mut context).is_pending() { }
    }
}

