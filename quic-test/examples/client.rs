use std::net::SocketAddr;

use anyhow::{anyhow, Result};
use s2n_quic::client::Connect;
use s2n_quic::Client;

const CERT_PEM: &str = include_str!("../fixtures/cert.pem");

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .with_tls(CERT_PEM)?
        .with_io("0.0.0.0:0")?
        .start()
        .map_err(|e| anyhow!("Failed to connect server: {e}"))?;

    let addr: SocketAddr = "127.0.0.1:4433".parse()?;
    let connect = Connect::new(addr).with_server_name("localhost");
    // 不能自动补全了，连类型都没法分析出来了
    // 貌似是 rust analyzer 的问题
    // 参考：https://github.com/rust-lang/rust-analyzer/issues/12759
    let mut conn = client.connect(connect).await?;

    conn.keep_alive(true)?;

    let stream = conn.open_bidirectional_stream().await?;
    let (mut recv_stream, mut send_stream) = stream.split();

    tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        if let Err(e) = tokio::io::copy(&mut recv_stream, &mut stdout).await {
            println!("Failed to copy data from server: {e}");
        }
    });

    let mut stdin = tokio::io::stdin();
    tokio::io::copy(&mut stdin, &mut send_stream).await?;

    Ok(())
}
