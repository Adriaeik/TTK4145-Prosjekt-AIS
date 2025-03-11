#[tokio::main]
async fn main() {
    let session = zenoh::open(zenoh::Config::default()).await.unwrap();
    session.put("stor/elevator1", "value1").await.unwrap();
    session.close().await.unwrap();
}
