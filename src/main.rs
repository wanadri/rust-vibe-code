use rust_vibe_code::Store;

#[tokio::main]
async fn main() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let store = Store::connect(&redis_url, "rust-vibe-code")
        .await
        .expect("failed to connect to Redis");

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Listening on http://{addr}");
    axum::serve(listener, rust_vibe_code::app(store))
        .await
        .unwrap();
}
