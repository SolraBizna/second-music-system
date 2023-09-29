use ::switchyard::Switchyard;

use super::*;

impl<T: Send + 'static> TaskRuntime for Switchyard<T> {
    fn spawn_task(&self, kind: TaskType, task: impl Future<Output=()> + Send + 'static) {
        let priority = match kind {
            TaskType::BufferLoad => 0,
            TaskType::StreamLoad => 1,
            TaskType::StreamDecode => 2,
        };
        self.spawn(priority, task);
    }
}
