# async-runtime

简易 Rust 异步 Runtime，借鉴自 [night-cruise/async-runtime](https://github.com/night-cruise/async-runtime)

IO 多路复用部分采用 [smol-rs/polling](https://github.com/smol-rs/polling)

单线程架构，REACTOR 和 Executor 在同一个线程。

运行例子：

```sh
cargo run --example echo_server
```

开启运行后的 `echo server` 会监听地址：`127.0.0.1:8080`