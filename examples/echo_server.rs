use std::error::Error;

use async_runtime::reactor::react_and_run;

use async_runtime::{
    async_io::{TcpListener, TcpStream},
    executor::{spawner_and_executor, Spawner},
};

async fn handle_client(mut stream: TcpStream) {
    let mut buf = [0u8; 1024];
    println!("handle client {}", stream.raw_key());
    loop {
        let n = stream.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }
        stream.write(&buf[..n]).await.unwrap();
    }
}

async fn server_loop(spawner: Spawner) {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("bind addr 127.0.0.1:8080 {}", listener.raw_key());

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        println!("receive {}", stream.raw_key());
        spawner.spawn(handle_client(stream));
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let (spawner, executor) = spawner_and_executor();

    spawner.spawn(server_loop(spawner.clone()));

    react_and_run(executor)
}
