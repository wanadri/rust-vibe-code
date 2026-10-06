use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub completed: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTodo {
    pub title: String,
    pub completed: bool,
}

/// Redis-backed todo storage.
///
/// Todos live as JSON in the hash `{prefix}:todos` (field = id), and ids
/// come from the counter `{prefix}:next_id`.
#[derive(Clone)]
pub struct Store {
    conn: ConnectionManager,
    prefix: String,
}

impl Store {
    pub async fn connect(redis_url: &str, prefix: impl Into<String>) -> redis::RedisResult<Self> {
        let conn = redis::Client::open(redis_url)?
            .get_connection_manager()
            .await?;
        Ok(Self {
            conn,
            prefix: prefix.into(),
        })
    }

    fn todos_key(&self) -> String {
        format!("{}:todos", self.prefix)
    }

    fn next_id_key(&self) -> String {
        format!("{}:next_id", self.prefix)
    }

    /// Deletes every key owned by this store.
    pub async fn clear(&self) -> redis::RedisResult<()> {
        let mut conn = self.conn.clone();
        let _: () = conn.del(&[self.todos_key(), self.next_id_key()]).await?;
        Ok(())
    }
}

pub enum ApiError {
    NotFound,
    Validation(&'static str),
    Internal(String),
}

impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "todo not found"),
            ApiError::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            ApiError::Internal(err) => {
                eprintln!("internal error: {err}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

fn validate_title(title: &str) -> Result<String, ApiError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(ApiError::Validation("title must not be empty"));
    }
    Ok(title.to_string())
}

pub fn router(store: Store) -> Router {
    Router::new()
        .route("/todos", get(list_todos).post(create_todo))
        .route(
            "/todos/{id}",
            get(get_todo).put(update_todo).delete(delete_todo),
        )
        .with_state(store)
}

async fn list_todos(State(store): State<Store>) -> Result<Json<Vec<Todo>>, ApiError> {
    let mut conn = store.conn.clone();
    let values: Vec<String> = conn.hvals(store.todos_key()).await?;
    let mut todos = values
        .iter()
        .map(|v| serde_json::from_str(v))
        .collect::<Result<Vec<Todo>, _>>()?;
    todos.sort_by_key(|t| t.id);
    Ok(Json(todos))
}

async fn create_todo(
    State(store): State<Store>,
    Json(input): Json<CreateTodo>,
) -> Result<(StatusCode, Json<Todo>), ApiError> {
    let title = validate_title(&input.title)?;
    let mut conn = store.conn.clone();
    let id: u64 = conn.incr(store.next_id_key(), 1).await?;
    let todo = Todo {
        id,
        title,
        completed: false,
    };
    let _: () = conn
        .hset(store.todos_key(), id, serde_json::to_string(&todo)?)
        .await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

async fn get_todo(State(store): State<Store>, Path(id): Path<u64>) -> Result<Json<Todo>, ApiError> {
    let mut conn = store.conn.clone();
    let value: Option<String> = conn.hget(store.todos_key(), id).await?;
    let value = value.ok_or(ApiError::NotFound)?;
    Ok(Json(serde_json::from_str(&value)?))
}

async fn update_todo(
    State(store): State<Store>,
    Path(id): Path<u64>,
    Json(input): Json<UpdateTodo>,
) -> Result<Json<Todo>, ApiError> {
    let title = validate_title(&input.title)?;
    let mut conn = store.conn.clone();
    let exists: bool = conn.hexists(store.todos_key(), id).await?;
    if !exists {
        return Err(ApiError::NotFound);
    }
    let todo = Todo {
        id,
        title,
        completed: input.completed,
    };
    let _: () = conn
        .hset(store.todos_key(), id, serde_json::to_string(&todo)?)
        .await?;
    Ok(Json(todo))
}

async fn delete_todo(
    State(store): State<Store>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    let mut conn = store.conn.clone();
    let removed: usize = conn.hdel(store.todos_key(), id).await?;
    if removed == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
