# rust-vibe-code

A playground for learning Rust: a simple Todo CRUD API built with [Axum](https://github.com/tokio-rs/axum) and stored in [Redis](https://redis.io).

## Getting started

Install Rust via [rustup](https://rustup.rs) and start a Redis server, e.g.:

```sh
docker run -d -p 6379:6379 redis:7-alpine   # or: sudo apt install redis-server
```

Then:

```sh
cargo run      # start the API on http://localhost:3000
cargo test     # run tests (needs Redis running)
cargo fmt      # format code
cargo clippy   # lint
```

## API

Configuration (environment variables):

- `REDIS_URL` – Redis connection URL (default `redis://127.0.0.1:6379`)
- `PORT` – HTTP port (default `3000`)

Todos are stored as JSON in the Redis hash `rust-vibe-code:todos` (field = id); ids come from the counter `rust-vibe-code:next_id`.

| Method | Path          | Body                                   | Success |
| ------ | ------------- | -------------------------------------- | ------- |
| POST   | `/todos`      | `{"title": "..."}`                     | 201     |
| GET    | `/todos`      |                                        | 200     |
| GET    | `/todos/{id}` |                                        | 200     |
| PUT    | `/todos/{id}` | `{"title": "...", "completed": true}`  | 200     |
| DELETE | `/todos/{id}` |                                        | 204     |

Errors return JSON like `{"error": "todo not found"}` (404) or `{"error": "title must not be empty"}` (422). Redis failures return 500.

```sh
curl -X POST localhost:3000/todos -H 'content-type: application/json' -d '{"title":"Learn Rust"}'
curl localhost:3000/todos
curl localhost:3000/todos/1
curl -X PUT localhost:3000/todos/1 -H 'content-type: application/json' -d '{"title":"Learn Axum","completed":true}'
curl -X DELETE localhost:3000/todos/1
```

## Layout

- `src/main.rs` – starts the HTTP server
- `src/lib.rs` – builds the router
- `src/todos.rs` – Todo model, Redis store, and handlers
- `tests/api.rs` – integration tests for every endpoint (run against Redis)
