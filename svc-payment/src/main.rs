use std::env;

use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dto::*;
use routes::*;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

mod dto;
mod logger;
mod routes;
mod schema;

#[cfg(test)]
mod tests;

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(post_payment, delete_payment, get_payment),
    components(schemas(PaymentStatus, Payment, PaymentRequest))
)]
struct ApiDoc;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
pub const SERVICE_ENDPOINT: &str = "0.0.0.0:8060";

#[derive(Debug, Clone)]
struct AppState {
    database_url: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL environment variable was not specified");

    let _logger_handler = logger::init();
    log::debug!("Logger initialized. Hello, world!");

    let app = app(database_url).await;

    log::info!("Listening on {}", SERVICE_ENDPOINT);
    let listener = TcpListener::bind(SERVICE_ENDPOINT).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn app(database_url: String) -> axum::Router {
    init_db(database_url.as_str());

    let swagger = SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi());
    let state = AppState { database_url };
    let app = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(routes::check_health))
        .routes(routes!(routes::post_payment))
        .routes(routes!(routes::get_payment, routes::delete_payment))
        .with_state(state);

    axum::Router::from(app).merge(swagger)
}

fn init_db(database_url: &str) {
    let conn = &mut PgConnection::establish(database_url)
        .expect("Failed to establish connection to database");
    let result = conn.run_pending_migrations(MIGRATIONS);
    if let Err(e) = result {
        panic!("Failed to initialize DB: {e}");
    }
}
