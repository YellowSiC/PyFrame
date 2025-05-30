// Copyright 2025-2030 PyFrame Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{
    lock_force, log_if_err,
    utils::{arc_mut, ArcMut},
};
use anyhow::{anyhow, Result};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

type Task = Box<dyn FnOnce() -> Result<()> + Send + 'static>;

#[derive(Debug)]
pub struct ThreadPool {
    sender: mpsc::Sender<Task>,
}

impl ThreadPool {
    pub fn new(size: u32) -> ArcMut<ThreadPool> {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let mut workers = Vec::with_capacity(size as usize);

        let receiver = arc_mut(receiver);
        for _ in 0..size {
            workers.push(Self::create_worker(receiver.clone()));
        }

        arc_mut(ThreadPool { sender })
    }

    pub fn run<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce() -> Result<()> + Send + 'static,
    {
        let job = Box::new(f);
        self.sender
            .send(job)
            .map_err(|_| anyhow!("Failed to send job to thread pool"))
    }

    fn create_worker(receiver: ArcMut<Receiver<Task>>) -> thread::JoinHandle<()> {
        std::thread::spawn(move || {
            loop {
                let job = lock_force!(receiver).recv();
                // unlock receiver before job start
                log_if_err!(job);
                if let Ok(job) = job {
                    log_if_err!(job());
                }
            }
        })
    }
}
