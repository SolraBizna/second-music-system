use ::tokio::runtime::Runtime;

use super::*;

impl TaskRuntime for Runtime {
    fn spawn_task(&self, _kind: TaskType, task: impl Future<Output=()> + Send + 'static) {
        self.spawn(task);
    }
}
