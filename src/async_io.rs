use std::{
    cell::RefCell,
    future::Future,
    io::{self, Read, Write},
    net::SocketAddr,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    task::{Context, Poll},
};

use polling::Event;

use crate::reactor::REACTOR;

struct Key(usize);

impl Key {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Key(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct TcpListener {
    key: Key,
    inner: Mutex<std::net::TcpListener>,
}

pub struct TcpStream {
    key: Key,
    inner: Mutex<std::net::TcpStream>,
}

pub struct AcceptFuture<'a>(&'a TcpListener);
pub struct ReadFuture<'a>(&'a mut TcpStream, RefCell<&'a mut [u8]>);
pub struct WriteFuture<'a>(&'a mut TcpStream, RefCell<&'a [u8]>);

impl TcpListener {
    fn new(listener: std::net::TcpListener) -> Self {
        Self {
            key: Key::new(),
            inner: Mutex::new(listener),
        }
    }

    pub fn bind<A: std::net::ToSocketAddrs>(addr: A) -> std::io::Result<TcpListener> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(TcpListener::new(listener))
    }

    pub fn accept(&self) -> AcceptFuture<'_> {
        AcceptFuture(self)
    }

    pub fn raw_key(&self) -> usize {
        self.key.0
    }
}

impl TcpStream {
    fn new(stream: std::net::TcpStream) -> Self {
        Self {
            key: Key::new(),
            inner: Mutex::new(stream),
        }
    }

    pub fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFuture<'a> {
        ReadFuture(self, RefCell::new(buf))
    }

    pub fn write<'a>(&'a mut self, buf: &'a [u8]) -> WriteFuture<'a> {
        WriteFuture(self, RefCell::new(buf))
    }

    pub fn raw_key(&self) -> usize {
        self.key.0
    }
}

impl Future for AcceptFuture<'_> {
    type Output = std::io::Result<(TcpStream, SocketAddr)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.0.inner.lock().unwrap();
        match inner.accept() {
            Ok((stream, addr)) => {
                REACTOR.delete_event(inner.deref())?;
                stream.set_nonblocking(true)?;
                Poll::Ready(Ok((TcpStream::new(stream), addr)))
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                REACTOR.add_event(
                    Event::readable(self.0.key.0),
                    inner.deref(),
                    cx.waker().clone(),
                )?;
                Poll::Pending
            }
            Err(e) => {
                REACTOR.delete_event(inner.deref())?;
                Poll::Ready(Err(e))
            }
        }
    }
}

impl Future for ReadFuture<'_> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.0.inner.lock().unwrap();
        match inner.read(self.1.borrow_mut().deref_mut()) {
            Ok(n) => {
                REACTOR.delete_event(inner.deref())?;
                Poll::Ready(Ok(n))
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                REACTOR.add_event(
                    Event::readable(self.0.key.0),
                    inner.deref(),
                    cx.waker().clone(),
                )?;
                Poll::Pending
            }
            Err(e) => {
                REACTOR.delete_event(inner.deref())?;
                Poll::Ready(Err(e))
            }
        }
    }
}

impl Future for WriteFuture<'_> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.0.inner.lock().unwrap();
        match inner.write(self.1.borrow().deref()) {
            Ok(n) => {
                REACTOR.delete_event(inner.deref())?;
                Poll::Ready(Ok(n))
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                REACTOR.add_event(
                    Event::writable(self.0.key.0),
                    inner.deref(),
                    cx.waker().clone(),
                )?;
                Poll::Pending
            }
            Err(e) => {
                REACTOR.delete_event(inner.deref())?;
                Poll::Ready(Err(e))
            }
        }
    }
}
