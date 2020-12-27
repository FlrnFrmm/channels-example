#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use std::{thread, time};
use rand::prelude::*;
use crossbeam_channel::{select, unbounded, Sender, Receiver};
use anyhow::Result;

struct CollectorChannel {
    sender: Sender<()>,
    receiver: Receiver<(u64, u64)>
}

impl CollectorChannel {
    pub fn new() -> Self {
        let (sender, receiver) = init_collector_thread();
        Self { sender, receiver }
    }

    pub fn get_current_value(&self) -> Result<(u64, u64)> {
        self.sender.send(())?;
        Ok(self.receiver.recv()?)
    }
}

#[get("/")]
fn info(collector_channel: rocket::State<CollectorChannel>) -> String {
    match collector_channel.get_current_value() {
        Ok((value_a, value_b)) => format!("Value A: {}\nValue B: {}", value_a, value_b),
        Err(_) => "Something went wrong pulling the value from the channel".into()
    }
}

fn main() {
    rocket::ignite()
        .mount("/", routes![info])
        .manage(CollectorChannel::new())
        .launch();
}

fn init_workers() -> (Sender<()>, Receiver<(u64, u64)>){
    let (collector_sender, receiver) = unbounded();
    let (trigger, collector_receiver) = unbounded();

    thread::spawn(move || {
        generate_worker_threads(collector_sender, collector_receiver);
    });

    (trigger, receiver)
}

fn generate_worker_threads(sender: Sender<(u64, u64)>, receiver: Receiver<()>) {
    let (sender_worker_a, receiver_worker_a) = unbounded();
    let (sender_worker_b, receiver_worker_b) = unbounded();

    generate_worker_thread(sender_worker_a);
    generate_worker_thread(sender_worker_b);

    let (mut last_a, mut last_b) = (0, 0);

    loop {
        select!{
            recv(receiver_worker_a) -> msg => {
                if let Ok(value_a) = msg {
                    last_a = value_a;
                }
            },
            recv(receiver_worker_b) -> msg => {
                if let Ok(value_b) = msg {
                    last_b = value_b;
                }
            },
            recv(receiver) -> _ => sender.send((last_a, last_b)).unwrap()
        }
    }
}

fn generate_worker_thread(sender: Sender<u64>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let mut total_count = 0;
        loop {
            let seconds_to_wait = rng.next_u64() % 5 + 1;
            total_count += seconds_to_wait;
            thread::sleep(time::Duration::from_secs(seconds_to_wait));
            sender.send(total_count).unwrap();
        }
    })
}
