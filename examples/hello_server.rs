use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

extern crate shared_channel;
use shared_channel::{shared_channel, SharedReceiver};

fn hello(id: u32, stream: TcpStream) {
    let reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);

    // read request
    for line in reader.lines() {
        match line {
            Ok(line) => {
                if line.len() == 0 {
                    break;
                }
            }
            _ => {}
        }
    }
    // send response
    write!(writer, "HTTP/1.1 200 OK\r\n\r\nHello! I'm No.{}", id).unwrap();
}

fn worker(id: u32, chan: SharedReceiver<TcpStream>) {
    loop {
        for steam in chan.iter() {
            hello(id, steam);
        }
    }
}

fn main() {
    const N_WORKER: u32 = 4;
    let (tx, rx) = shared_channel();

    // make thread pool
    for i in 0..N_WORKER {
        let rx = rx.clone();
        thread::spawn(move || worker(i, rx));
    }

    // run server forever
    let listener = TcpListener::bind("localhost:5000").expect("failed to listen port 5000");
    loop {
        match listener.accept() {
            Ok((stream, addr)) => {
                println!("new client: {}", addr);
                tx.send(stream).unwrap();
            }
            Err(e) => println!("accept error: {}", e),
        }
    }
}
