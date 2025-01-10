use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use diesel::{prelude::*, result::Error as DieselError};
use uuid::Uuid;

use crate::{dto::*, schema::payment, AppState};

#[utoipa::path(
    get,
    path = "/manage/health",
    responses(
        (status = OK, description = "Success")
    )
)]
pub async fn check_health() -> impl IntoResponse {
    StatusCode::OK
}

#[utoipa::path(
    get,
    path = "/api/v1/payment/{paymentUid}",
    responses(
        (
            status = OK,
            description = "Данные оплаты",
            body = Payment,
            content_type = "application/json",
        ),
    ),
    params(
        ("paymentUid", Path, description = "Идентификатор оплаты")
    ),
)]
pub async fn get_payment(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");

    let res = payment::table
        .filter(payment::payment_uid.eq(uid))
        .select(Payment::as_select())
        .get_result::<Payment>(conn)
        .map_err(|e| match e {
            DieselError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(Json(res))
}

#[utoipa::path(
    delete,
    path = "/api/v1/payment/{paymentUid}",
    responses(
        (
            status = NO_CONTENT,
            description = "Оплата отменена",
            content_type = "application/json",
        ),
    ),
    params(
        ("paymentUid", Path, description = "Идентификатор оплаты")
    ),
)]
pub async fn delete_payment(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");

    diesel::update(payment::table)
        .filter(payment::payment_uid.eq(uid))
        .set(payment::status.eq(PaymentStatus::Canceled.to_string()))
        .execute(conn)
        .map_err(|e| match e {
            DieselError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/payment",
    responses(
        (status = CREATED, body = Payment, description = "Success")
    ),
)]
pub async fn post_payment(
    State(state): State<AppState>,
    Json(payment): Json<PaymentRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");

    let payment = Payment::from(payment);
    let created = diesel::insert_into(payment::table)
        .values(&payment)
        .returning(Payment::as_returning())
        .get_result(conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log::debug!("Created payment: {}", created.payment_uid);

    Ok((StatusCode::CREATED, Json(created)))
}
