use std::{collections::BTreeMap, error::Error, io, sync::Mutex, task::Waker};

use polling::{AsRawSource, AsSource, Event, Events, Poller};

use lazy_static::lazy_static;

use crate::executor::Executor;

lazy_static! {
    pub(crate) static ref REACTOR: Reactor = {
        Reactor {
            poller: Poller::new().expect("poller create failed"),
            wakers: Mutex::new(BTreeMap::new()),
        }
    };
}

pub(crate) struct Reactor {
    pub poller: Poller,
    pub wakers: Mutex<BTreeMap<usize, Waker>>,
}

impl Reactor {
    pub(crate) fn add_event(
        &self,
        event: Event,
        source: impl AsRawSource,
        waker: Waker,
    ) -> io::Result<()> {
        unsafe { self.poller.add(source, event)? }
        self.wakers.lock().unwrap().insert(event.key, waker);
        Ok(())
    }

    pub(crate) fn delete_event(&self, source: impl AsSource) -> io::Result<()> {
        self.poller.delete(&source)
    }

    pub(crate) fn react(self: &Self, events: &mut Events) -> io::Result<()> {
        events.clear();
        self.poller.wait(events, None)?;
        for ev in events.iter() {
            if let Some(waker) = self.wakers.lock().unwrap().remove(&ev.key) {
                waker.wake();
            }
        }
        Ok(())
    }
}

pub fn react_and_run(mut executor: Executor) -> Result<(), Box<dyn Error>> {
    let mut events = Events::new();
    loop {
        executor.run()?;
        REACTOR.react(&mut events)?;
    }
}
