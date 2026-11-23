use super::task::Task;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Queue {
    tasks: VecDeque<Task>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }

    pub fn push(&mut self, task: Task) {
        self.tasks.push_back(task);
    }

    pub fn pop(&mut self) -> Option<Task> {
        self.tasks.pop_front()
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}
