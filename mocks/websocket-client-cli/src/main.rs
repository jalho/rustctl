use futures_util::stream::{SplitSink, SplitStream};
use tokio_tungstenite::tungstenite;

fn main() {
    let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(work_with_websocket());
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type UpgradeResponse = tungstenite::http::Response<Option<Vec<u8>>>;

async fn work_with_websocket() {
    let (socket, _response): (Socket, UpgradeResponse) =
        tokio_tungstenite::connect_async("ws://127.0.0.1:8080/ws")
            .await
            .unwrap();
    let (sock_w, sock_r): (SplitSink<Socket, tungstenite::Message>, SplitStream<Socket>) =
        futures_util::StreamExt::split(socket);
    let coroutine_read = tokio::spawn(work_with_readable(sock_r));
    let coroutine_write = tokio::spawn(work_with_writeable(sock_w));
    let (coroutine_read, coroutine_write) = tokio::join!(coroutine_read, coroutine_write);
    coroutine_read.unwrap();
    coroutine_write.unwrap();
}

async fn work_with_readable(mut readable: SplitStream<Socket>) {
    loop {
        match futures_util::StreamExt::next(&mut readable).await {
            Some(Ok(msg)) => {
                dbg!(msg);
            }
            Some(Err(e)) => {
                eprintln!("Error receiving message: {e}");
                break;
            }
            None => {
                println!("Connection closed");
                break;
            }
        }
    }
}

async fn work_with_writeable(mut writable: SplitSink<Socket, tungstenite::Message>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));

    loop {
        interval.tick().await;
        let msg = tungstenite::Message::Text("Hello to server from CLI client!".into());
        if let Err(e) = futures_util::SinkExt::send(&mut writable, msg).await {
            eprintln!("Error sending message: {e}");
            break;
        }
    }
}
