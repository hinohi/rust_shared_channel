# shared channel

This is a implementation of `multi-producer, multi-consumer channel` in Rust.

## Example

### simple usage

```rust
use std::thread;

extern crate shared_channel;
use shared_channel::shared_channel;

fn main() {
    let (tx, rx) = shared_channel();
    for i in 0..10 {
        let rx = rx.clone();
        thread::spawn(move || println!("{}", rx.recv().unwrap()));
    }
    for i in 0..10 {
        tx.send(i).unwrap();
    }
}
```

### Pre-forked web server

```sh
git clone https://github.com/hinohi/rust_shared_channel.git
cd rust_shared_channel
cargo run --example hello_server
```

Please access `http://localhost:5000`.
Its response looks like `Hello! I'm No.0`.
The number shows ID of thread-pool's worker.

## TODO

* [ ] implement `Reciver`'s stable API
    * [x] `try_recv`
    * [x] `recv`
    * [ ] `recv_timeout`
    * [x] `iter`
    * [ ] `try_iter`
* [ ] implement `shared_sync_channel`
