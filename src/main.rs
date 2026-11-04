use axum::{
    routing::{get, post},
    Router,
    Json,
    http::{StatusCode, header},
    response::{Html, IntoResponse},
    Form,
    extract::{Path, FromRequestParts},
};
use axum::http::request::Parts;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, FromRow};
use std::sync::Arc;
use dotenvy::dotenv;
use askama::Template;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use bcrypt::{hash, verify, DEFAULT_COST};

#[derive(Clone, Serialize, Deserialize, FromRow)]
struct Asset {
    id: i64,
    name: String,
    symbol: String,
    quantity: f64,
    price_usd: f64,
}

#[derive(Clone, FromRow)]
struct User {
    id: i64,
    email: String,
    password_hash: String,
}

#[derive(Deserialize)]
struct AuthPayload {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct AssetForm {
    name: String,
    symbol: String,
    quantity: f64,
    price_usd: f64,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

struct AuthenticatedUser(Claims);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let token = &auth_header[7..];
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret_default".to_string());

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &Validation::default(),
        ).map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthenticatedUser(token_data.claims))
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    assets: Vec<Asset>,
    total_portfolio: f64,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {}

struct AppState {
    db: SqlitePool,
    jwt_secret: String,
}

type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() {
    dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL obrigatória");
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret_default".to_string());
    
    let db = SqlitePool::connect(&database_url).await.unwrap();
    
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Erro ao rodar migrações");

    let state = SharedState::new(AppState { db, jwt_secret });

    let app = Router::new()
        .route("/", get(index_page))
        .route("/login", get(login_page).post(login_handler))
        .route("/register", get(register_page).post(register_handler))
        .route("/assets/create", post(create_asset_web))
        .route("/assets/delete/:id", post(delete_asset_web))
        .route("/api/assets", get(list_assets).post(create_asset_api))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Servidor rodando em http://127.0.0.1:3000 com sucesso!");
    
    axum::serve(listener, app).await.unwrap();
}

async fn login_page() -> impl IntoResponse {
    let template = LoginTemplate {};
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn register_page() -> impl IntoResponse {
    let template = RegisterTemplate {};
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn register_handler(
    axum::extract::State(state): axum::extract::State<SharedState>,
    Form(payload): Form<AuthPayload>,
) -> impl IntoResponse {
    let password_hash = match hash(payload.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let result = sqlx::query("INSERT INTO users (email, password_hash) VALUES (?, ?)")
        .bind(&payload.email)
        .bind(password_hash)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => Html("Utilizador registado com sucesso! <a href='/login'>Faça login</a>").into_response(),
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn login_handler(
    axum::extract::State(state): axum::extract::State<SharedState>,
    Form(payload): Form<AuthPayload>,
) -> impl IntoResponse {
    let user = sqlx::query_as::<_, User>("SELECT id, email, password_hash FROM users WHERE email = ?")
        .bind(&payload.email)
        .fetch_optional(&state.db)
        .await;

    match user {
        Ok(Some(u)) => {
            if verify(&payload.password, &u.password_hash).unwrap_or(false) {
                let expiration = chrono::Utc::now()
                    .checked_add_signed(chrono::Duration::hours(24))
                    .expect("data válida")
                    .timestamp() as usize;
                
                let claims = Claims { sub: u.email, exp: expiration };
                let token = encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(state.jwt_secret.as_bytes())
                ).unwrap();

                format!("Login bem-sucedido!\nSeu token JWT:\n{}", token).into_response()
            } else {
                StatusCode::UNAUTHORIZED.into_response()
            }
        }
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

async fn index_page(
    axum::extract::State(state): axum::extract::State<SharedState>,
) -> impl IntoResponse {
    let assets = sqlx::query_as::<_, Asset>("SELECT id, name, symbol, quantity, price_usd FROM assets")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let total_portfolio: f64 = assets.iter().map(|a| a.quantity * a.price_usd).sum();

    let template = IndexTemplate { assets, total_portfolio };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn create_asset_web(
    axum::extract::State(state): axum::extract::State<SharedState>,
    Form(payload): Form<AssetForm>,
) -> impl IntoResponse {
    let _ = sqlx::query(
        "INSERT INTO assets (name, symbol, quantity, price_usd) VALUES (?, ?, ?, ?)"
    )
    .bind(payload.name)
    .bind(payload.symbol)
    .bind(payload.quantity)
    .bind(payload.price_usd)
    .execute(&state.db)
    .await;

    axum::response::Redirect::to("/").into_response()
}

async fn delete_asset_web(
    axum::extract::State(state): axum::extract::State<SharedState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let _ = sqlx::query("DELETE FROM assets WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await;

    axum::response::Redirect::to("/").into_response()
}

async fn list_assets(
    _auth: AuthenticatedUser,
    axum::extract::State(state): axum::extract::State<SharedState>,
) -> Result<Json<Vec<Asset>>, StatusCode> {
    let assets = sqlx::query_as::<_, Asset>("SELECT id, name, symbol, quantity, price_usd FROM assets")
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(assets))
}

async fn create_asset_api(
    _auth: AuthenticatedUser,
    axum::extract::State(state): axum::extract::State<SharedState>,
    Json(payload): Json<AssetForm>,
) -> StatusCode {
    let result = sqlx::query(
        "INSERT INTO assets (name, symbol, quantity, price_usd) VALUES (?, ?, ?, ?)"
    )
    .bind(payload.name)
    .bind(payload.symbol)
    .bind(payload.quantity)
    .bind(payload.price_usd)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => StatusCode::CREATED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}