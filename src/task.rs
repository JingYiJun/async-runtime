use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::SyncSender,
        Arc, Mutex,
    },
    task::{Context, Poll, Wake},
};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct TaskId(u64);

impl TaskId {
    pub(crate) fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub(crate) struct Task {
    id: TaskId,
    future: Mutex<Pin<Box<dyn Future<Output = ()> + 'static + Send>>>,
    task_sender: SyncSender<Arc<Task>>,
}

impl Task {
    pub(crate) fn new(
        future: impl Future<Output = ()> + 'static + Send,
        task_sender: SyncSender<Arc<Task>>,
    ) -> Self {
        Task {
            id: TaskId::new(),
            future: Mutex::new(Box::pin(future)),
            task_sender,
        }
    }

    pub(crate) fn poll(&self, context: &mut Context) -> Poll<()> {
        self.future
            .lock()
            .expect("get lock failed")
            .as_mut()
            .poll(context)
    }

    pub(crate) fn task_id(&self) -> TaskId {
        self.id
    }
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.task_sender
            .send(self.clone())
            .expect("send task failed")
    }
}
