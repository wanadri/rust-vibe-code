use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rust_vibe_code::Store;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Connects to Redis under a key prefix unique to `test_name`, so tests can run in parallel.
async fn test_store(test_name: &str) -> Store {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let prefix = format!("test:{}:{test_name}", std::process::id());
    let store = Store::connect(&url, prefix)
        .await
        .expect("Redis must be running for tests");
    store.clear().await.unwrap();
    store
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(body) => builder
            .header("content-type", "application/json")
            .body(Body::from(body.to_string())),
        None => builder.body(Body::empty()),
    }
    .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json)
}

#[tokio::test]
async fn crud_lifecycle() {
    let store = test_store("crud_lifecycle").await;
    let app = rust_vibe_code::app(store.clone());

    let (status, body) = send(&app, "GET", "/todos", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));

    let (status, body) = send(
        &app,
        "POST",
        "/todos",
        Some(json!({ "title": "Learn Rust" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        body,
        json!({ "id": 1, "title": "Learn Rust", "completed": false })
    );

    let (status, body) = send(&app, "GET", "/todos/1", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["title"], "Learn Rust");

    let (status, body) = send(
        &app,
        "PUT",
        "/todos/1",
        Some(json!({ "title": "Learn Axum", "completed": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({ "id": 1, "title": "Learn Axum", "completed": true })
    );

    let (status, body) = send(&app, "GET", "/todos", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);

    let (status, _) = send(&app, "DELETE", "/todos/1", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = send(&app, "GET", "/todos/1", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({ "error": "todo not found" }));

    store.clear().await.unwrap();
}

#[tokio::test]
async fn missing_todo_returns_404() {
    let store = test_store("missing_todo_returns_404").await;
    let app = rust_vibe_code::app(store.clone());
    let update = json!({ "title": "x", "completed": false });

    assert_eq!(
        send(&app, "GET", "/todos/42", None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "PUT", "/todos/42", Some(update)).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "DELETE", "/todos/42", None).await.0,
        StatusCode::NOT_FOUND
    );

    store.clear().await.unwrap();
}

#[tokio::test]
async fn empty_title_is_rejected() {
    let store = test_store("empty_title_is_rejected").await;
    let app = rust_vibe_code::app(store.clone());

    let (status, body) = send(&app, "POST", "/todos", Some(json!({ "title": "   " }))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body, json!({ "error": "title must not be empty" }));

    store.clear().await.unwrap();
}
