use std::{pin::Pin, time::Duration};

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use dto::*;
use futures::Future;
use routes::*;
use tokio::{net::TcpListener, sync::mpsc};
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

pub const MESSAGE_QUEUE_SIZE: usize = 10;

#[derive(Debug, Clone)]
struct AppState {
    msg_chan: mpsc::Sender<Message>,
}

pub type RequestReturnValue = Pin<Box<dyn Future<Output = Result<(), StatusCode>> + Send>>;
pub type RequestFn =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), StatusCode>> + Send>> + Send>;

struct Message {
    timeout: DateTime<Utc>,
    request: RequestFn,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let _logger_handler = logger::init();
    log::debug!("Logger initialized. Hello, world!");

    let (w, r) = mpsc::channel(MESSAGE_QUEUE_SIZE);
    let app = app(w).await;

    log::info!("Listening on {}", SERVICE_ENDPOINT);
    let listener = TcpListener::bind(SERVICE_ENDPOINT).await.unwrap();

    let sender_handle = tokio::spawn(queue_sender(r));

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    let (r,) = tokio::join!(sender_handle);
    r.expect("Failed to join sender handle");
}

async fn app(msg_chan: mpsc::Sender<Message>) -> axum::Router {
    let swagger = SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi());
    let state = AppState { msg_chan };
    let app = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(check_health))
        .routes(routes!(get_hotels))
        .routes(routes!(get_loyalty))
        .routes(routes!(get_reservations, post_reservation))
        .routes(routes!(delete_reservation, get_reservation))
        .routes(routes!(get_me))
        .with_state(state);

    axum::Router::from(app).merge(swagger)
}

async fn queue_sender(mut recv: mpsc::Receiver<Message>) {
    while let Some(m) = recv.recv().await {
        loop {
            match (m.request)().await {
                Err(s) => {
                    log::debug!("Failed retry with status {s}, sleeping for 500ms");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                Ok(_) => {
                    log::debug!("Successfully sent queued request");
                    break;
                }
            }
            if m.timeout < Utc::now() {
                log::warn!("Queued request timeout");
                break;
            }
        }
    }
}
