pub mod todos;

use axum::Router;

pub use todos::Store;

/// Builds the application router backed by the given Redis store.
pub fn app(store: Store) -> Router {
    todos::router(store)
}
