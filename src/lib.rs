//! Multi-producer, multi-consumer FIFO queue communication primitives.
//!
//! This module is extension of `std::sync::mpsc`, almost has same API
//! with it.
//! Differences are:
//!
//! * A struct [`SharedReceiver`] is defined. This is clone-able struct
//!   (multi-consumer).
//! * A function [`shared_channel`] corresponding to function `channel`
//!   is defined. [`shared_channel`] returns a `(Sender, SharedReceiver)`
//!   tuple instead of `(Sender, Receiver)` tuple.
//! * Some feature of `std::sync::mpsc` is not implemented yet.
//!
//! [`SharedReceiver`]: struct.SharedReceiver.html
//! [`shared_channel`]: fn.shared_channel.html
//!
//! # Example
//!
//! Simple usage:
//!
//! ```rust
//! # use std::thread;
//! # extern crate shared_channel;
//! # use shared_channel::shared_channel;
//! # fn main() {
//! let (tx, rx) = shared_channel();
//! for i in 0..10 {
//!     let rx = rx.clone();
//!     thread::spawn(move || println!("{}", rx.recv().unwrap()));
//! }
//!
//! for i in 0..10 {
//!     tx.send(i).unwrap();
//! }
//! # }
//! ```
//!
//! More examples, see examples directory.

use std::sync::mpsc::{channel, Receiver, RecvError, Sender, TryRecvError};
use std::sync::{Arc, Mutex, TryLockError};

pub struct SharedReceiver<T> {
    inner: Arc<Mutex<Receiver<T>>>,
}

pub struct Iter<'a, T: 'a> {
    rx: &'a SharedReceiver<T>,
}

impl<T> Clone for SharedReceiver<T> {
    fn clone(&self) -> Self {
        SharedReceiver {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> SharedReceiver<T> {
    fn new(receiver: Receiver<T>) -> SharedReceiver<T> {
        SharedReceiver {
            inner: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        match self.inner.try_lock() {
            Ok(mutex) => mutex.try_recv(),
            Err(TryLockError::Poisoned(_)) => Err(TryRecvError::Disconnected),
            _ => Err(TryRecvError::Empty),
        }
    }

    pub fn recv(&self) -> Result<T, RecvError> {
        match self.inner.lock() {
            Ok(mutex) => mutex.recv(),
            Err(_) => Err(RecvError),
        }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter { rx: self }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.rx.recv().ok()
    }
}

impl<'a, T> IntoIterator for &'a SharedReceiver<T> {
    type Item = T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

pub fn shared_channel<T>() -> (Sender<T>, SharedReceiver<T>) {
    let (sender, receiver) = channel();
    (sender, SharedReceiver::new(receiver))
}

#[cfg(test)]
mod tests {
    use super::shared_channel;
    use std::thread;

    #[test]
    fn smoke() {
        let (tx, rx) = shared_channel::<i32>();
        tx.send(1).unwrap();
        assert_eq!(rx.recv().unwrap(), 1);
    }

    #[test]
    fn smoke_multi_sender() {
        let (tx, rx) = shared_channel::<i32>();
        tx.send(1).unwrap();
        assert_eq!(rx.recv().unwrap(), 1);
        let tx = tx.clone();
        tx.send(1).unwrap();
        assert_eq!(rx.recv().unwrap(), 1);
    }

    #[test]
    fn smoke_multi_receiver() {
        let (tx, rx) = shared_channel::<i32>();
        let rx2 = rx.clone();
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        assert_eq!(rx.recv().unwrap(), 1);
        assert_eq!(rx2.recv().unwrap(), 2);
    }

    #[test]
    fn smoke_port_gone() {
        let (tx, rx) = shared_channel::<i32>();
        drop(rx);
        assert!(tx.send(1).is_err());
    }

    #[test]
    fn port_gone_concurrent() {
        let (tx, rx) = shared_channel::<i32>();
        let _t = thread::spawn(move || {
            rx.recv().unwrap();
            rx.recv().unwrap();
        });
        while tx.send(1).is_ok() {}
    }

    #[test]
    fn smoke_chan_gone() {
        let (tx, rx) = shared_channel::<i32>();
        drop(tx);
        assert!(rx.recv().is_err());
    }

    #[test]
    fn chan_gone_concurrent() {
        let (tx, rx) = shared_channel::<i32>();
        let _t = thread::spawn(move || {
            tx.send(1).unwrap();
            tx.send(1).unwrap();
        });
        while rx.recv().is_ok() {}
    }

    #[test]
    fn smoke_threads() {
        let (tx, rx) = shared_channel::<i32>();
        let _t = thread::spawn(move || {
            tx.send(1).unwrap();
        });
        assert_eq!(rx.recv().unwrap(), 1);
    }

    #[test]
    fn smoke_threads2() {
        let (tx, rx) = shared_channel::<i32>();
        let t = thread::spawn(move || {
            assert_eq!(rx.recv().unwrap(), 1);
        });
        tx.send(1).unwrap();
        t.join().ok().unwrap();
    }

    #[test]
    fn stress() {
        let (tx, rx) = shared_channel::<i32>();
        let t = thread::spawn(move || {
            for _ in 0..10000 {
                tx.send(1).unwrap();
            }
        });
        for _ in 0..10000 {
            assert_eq!(rx.recv().unwrap(), 1);
        }
        t.join().ok().unwrap();
    }

    #[test]
    fn stress_multi_sender() {
        const AMT: u32 = 10000;
        const N_THREADS: u32 = 8;
        let (tx, rx) = shared_channel::<i32>();

        let t = thread::spawn(move || {
            for _ in 0..AMT * N_THREADS {
                assert_eq!(rx.recv().unwrap(), 1);
            }
            match rx.try_recv() {
                Ok(..) => panic!(),
                _ => {}
            }
        });

        for _ in 0..N_THREADS {
            let tx = tx.clone();
            thread::spawn(move || {
                for _ in 0..AMT {
                    tx.send(1).unwrap();
                }
            });
        }
        drop(tx);
        t.join().ok().unwrap();
    }

    #[test]
    fn stress_multi_receiver() {
        const AMT: u32 = 10000;
        const N_THREADS: u32 = 8;
        let (tx, rx) = shared_channel::<i32>();

        let mut workers = Vec::new();
        for _ in 0..N_THREADS {
            let rx = rx.clone();
            let t = thread::spawn(move || {
                let mut count = 0;
                for _ in &rx {
                    count += 1;
                }
                count
            });
            workers.push(t);
        }

        for _ in 0..AMT * N_THREADS {
            tx.send(1).unwrap();
        }
        drop(tx);

        let mut count = 0;
        for t in workers {
            count += t.join().ok().unwrap();
        }
        assert_eq!(AMT * N_THREADS, count);
    }

    #[test]
    fn stress_multi() {
        const AMT: u32 = 10000;
        const N_SENDER: u32 = 4;
        const N_RECEIVER: u32 = 8;

        let (tx1, rx1) = shared_channel::<u32>();
        let (tx2, rx2) = shared_channel::<u32>();

        for _ in 0..N_RECEIVER {
            let rx1 = rx1.clone();
            let tx2 = tx2.clone();
            thread::spawn(move || {
                let mut sum = 0;
                for i in &rx1 {
                    sum += i;
                }
                tx2.send(sum).unwrap();
            });
        }

        let mut senders = Vec::new();
        for _ in 0..N_SENDER {
            let tx1 = tx1.clone();
            let t = thread::spawn(move || {
                for i in 1..AMT + 1 {
                    tx1.send(i).unwrap();
                }
            });
            senders.push(t);
        }
        drop(tx1);
        for t in senders {
            t.join().ok().unwrap();
        }

        let mut sum = 0;
        for _ in 0..N_RECEIVER {
            sum += rx2.recv().unwrap();
        }
        // Σ_{i=1}^{N} n = n (n + 1) / 2
        assert_eq!(AMT * (AMT + 1) / 2 * N_SENDER, sum);
    }

    #[test]
    fn smoke_try_recv() {
        let (tx, rx) = shared_channel::<i32>();
        let t = thread::spawn(move || {
            let mut sum = 0;
            loop {
                match rx.try_recv() {
                    Ok(i) => sum += i,
                    Err(_) => {}
                };
                if sum == 55 {
                    break;
                }
            }
        });
        for i in 1..10 + 1 {
            tx.send(i).unwrap();
        }
        t.join().ok().unwrap();
    }
}
