use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
};
use axum::{Extension, Json, Router, TypedHeader};
use jsonwebtoken as jwt;
use jwt::Validation;
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    let store = TodoStore::default();
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/todos", get(todos_handler).post(create_todo_handler))
        .layer(Extension(store))
        .route("/login", post(login_handler));

    // run it with hyper on localhost:3000
    let addr = "0.0.0.0:3000";
    println!("listening on http://{}", addr);
    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, Default, Clone)]
struct TodoStore {
    items: Arc<RwLock<Vec<Todo>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    id: usize,
    name: String,
    exp: usize,
}

#[async_trait]
impl<B> FromRequest<B> for Claims
where
    B: Send, // required by `async_trait`
{
    type Rejection = HttpError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request(req)
                .await
                .map_err(|_| HttpError::Auth)?;
        let key = jwt::DecodingKey::from_secret(SECRET);
        println!("{:?}", bearer.token());
        let token =
            jwt::decode::<Claims>(bearer.token(), &key, &Validation::default()).map_err(|e| {
                println!("{e:?}");
                HttpError::Auth
            })?;

        Ok(token.claims)
    }
}

#[derive(Debug)]
enum HttpError {
    Auth,
    Internal,
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        let (code, msg) = match self {
            HttpError::Auth => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            HttpError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Sever Error"),
        };

        (code, msg).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: usize,
    pub user_id: usize,
    pub title: String,
    pub completed: bool,
}

async fn index_handler() -> Html<&'static str> {
    Html("Hello, world!")
}

async fn todos_handler(
    claims: Claims,
    Extension(store): Extension<TodoStore>,
) -> Result<Json<Vec<Todo>>, HttpError> {
    match store.items.read() {
        Ok(items) => Ok(Json(
            items
                .iter()
                .filter(|todo| todo.user_id == claims.id)
                .cloned()
                .collect(),
        )),
        Err(_) => Err(HttpError::Internal),
    }
}

#[derive(Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

// eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpZCI6MSwibmFtZSI6ImxpbnV4ZmlzaCIsImV4cCI6MTY1OTA0Mzk4Nn0.foz4Zk5Zyo3mMkelp1DFS5V7hafRyobxGpngcrloybQ
// https://github.com/tokio-rs/axum/discussions/641
#[axum_macros::debug_handler]
async fn create_todo_handler(
    claims: Claims,
    Json(todo): Json<CreateTodo>,
    Extension(store): Extension<TodoStore>,
) -> Result<StatusCode, HttpError> {
    // TODO: validate token
    println!("claims: {:?}", claims);
    match store.items.write() {
        Ok(mut guard) => {
            let todo = Todo {
                id: get_next_id(),
                user_id: claims.id,
                title: todo.title,
                completed: false,
            };
            guard.push(todo);
            Ok(StatusCode::CREATED)
        }
        Err(_) => Err(HttpError::Internal),
    }
}

// 什么是 static？
// https://doc.rust-lang.org/reference/items/static-items.html
// 为啥不能用 const?
// https://rust-lang.github.io/rust-clippy/master/#declare_interior_mutable_const
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn get_next_id() -> usize {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginResponse {
    token: String,
}

const SECRET: &[u8] = b"deadbeef";

async fn login_handler(Json(_login): Json<LoginRequest>) -> Json<LoginResponse> {
    // TODO: do login validation
    let claims = Claims {
        id: 1,
        name: "linuxfish".to_string(),
        exp: get_epoch() + 14 * 3600,
    };

    let key = jwt::EncodingKey::from_secret(SECRET);
    let token = jwt::encode(&jwt::Header::default(), &claims, &key).unwrap();
    Json(LoginResponse { token })
}

fn get_epoch() -> usize {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
}
