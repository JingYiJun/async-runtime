use std::{
    collections::BTreeMap,
    error::Error,
    future::Future,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender, TryRecvError},
        Arc,
    },
    task::{Context, Poll, Waker},
};

use crate::task::{Task, TaskId};

pub struct Executor {
    task_receiver: Receiver<Arc<Task>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

#[derive(Clone)]
pub struct Spawner {
    task_sender: SyncSender<Arc<Task>>,
}

impl Executor {
    fn new(task_receiver: Receiver<Arc<Task>>) -> Self {
        Self {
            task_receiver,
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.task_receiver.try_recv() {
                Ok(task) => {
                    // wake
                    let waker = self
                        .waker_cache
                        .entry(task.task_id())
                        .or_insert_with(|| Waker::from(task.clone()));
                    let mut context = Context::from_waker(&waker);
                    match task.poll(&mut context) {
                        Poll::Ready(_) => {
                            self.waker_cache.remove(&task.task_id());
                        }
                        Poll::Pending => {}
                    }
                }
                Err(e) => match e {
                    TryRecvError::Empty => return Ok(()),
                    TryRecvError::Disconnected => return Err(Box::new(e)),
                },
            }
        }
    }
}

impl Spawner {
    fn new(task_sender: SyncSender<Arc<Task>>) -> Self {
        Self { task_sender }
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let task = Task::new(future, self.task_sender.clone());
        self.task_sender
            .send(Arc::new(task))
            .expect("send task failed");
    }
}

pub fn spawner_and_executor() -> (Spawner, Executor) {
    let (task_sender, task_receiver) = sync_channel(10000);
    (Spawner::new(task_sender), Executor::new(task_receiver))
}
