use std::env;

use dto::*;
use routes::*;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

mod dto;
mod logger;
mod routes;

#[cfg(test)]
mod tests;

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        check_health,
        get_me,
        get_hotels,
        get_loyalty,
        get_reservation,
        get_reservations,
        post_reservation,
        delete_reservation
    ),
    components(schemas(
        PaginationResponse,
        PaginationRequest,
        LoyaltyStatus,
        LoyaltyInfoResponse,
        PaymentInfo,
        PaymentStatus,
        HotelResponse,
        HotelInfo,
        UserInfoResponse,
        ReservationResponse,
        CreateReservationRequest,
        CreateReservationResponse
    ))
)]
struct ApiDoc;

pub const SERVICE_ENDPOINT: &str = "0.0.0.0:8080";
pub const RESERVATION_ENDPOINT: &str = "http://reservation:8070";
pub const PAYMENT_ENDPOINT: &str = "http://payment:8060";
pub const LOYALTY_ENDPOINT: &str = "http://loyalty:8050";
// pub const RESERVATION_ENDPOINT: &str = "http://localhost:8070";
// pub const PAYMENT_ENDPOINT: &str = "http://localhost:8060";
// pub const LOYALTY_ENDPOINT: &str = "http://localhost:8050";

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let _logger_handler = logger::init();
    log::debug!("Logger initialized. Hello, world!");

    let app = app().await;

    log::info!("Listening on {}", SERVICE_ENDPOINT);
    let listener = TcpListener::bind(SERVICE_ENDPOINT).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn app() -> axum::Router {
    let swagger = SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi());
    let app = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(check_health))
        .routes(routes!(get_hotels))
        .routes(routes!(get_loyalty))
        .routes(routes!(get_reservations, post_reservation))
        .routes(routes!(delete_reservation, get_reservation))
        .routes(routes!(get_me));

    axum::Router::from(app).merge(swagger)
}

