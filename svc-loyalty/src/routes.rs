use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use diesel::{prelude::*, result::Error as DieselError};

use crate::{dto::*, schema::loyalty, AppState};

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
    path = "/api/v1/loyalty",
    responses(
        (
            status = OK,
            description = "Данные программы лояльности",
            body = LoyaltyResponse,
            content_type = "application/json",
        ),
    ),
    params(
        ("X-User-Name", Header, description = "Имя пользователя")
    ),
)]
pub async fn get_loyalty(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");

    let res = loyalty::table
        .filter(loyalty::username.eq(username))
        .select(Loyalty::as_select())
        .get_result::<Loyalty>(conn)
        .map_err(|e| match e {
            DieselError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let res = LoyaltyResponse::from(res);

    Ok(Json(res))
}

#[utoipa::path(
    delete,
    path = "/api/v1/loyalty",
    responses(
        (
            status = NO_CONTENT,
            description = "Бронирование вычтено из программы лояльности",
            content_type = "application/json",
        ),
    ),
    params(
        ("X-User-Name", Header, description = "Имя пользователя")
    ),
)]
pub async fn delete_loyalty(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");

    let counter = diesel::update(loyalty::table)
        .filter(loyalty::username.eq(username))
        .set(loyalty::reservation_count.eq(loyalty::reservation_count - 1))
        .returning(loyalty::reservation_count)
        .get_result(conn)
        .map_err(|e| match e {
            DieselError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    if counter == 9 || counter == 19 {
        let (status, discount) = Loyalty::loyalty_from_counter(counter);

        diesel::update(loyalty::table)
            .filter(loyalty::username.eq(username))
            .set((loyalty::status.eq(status), loyalty::discount.eq(discount)))
            .execute(conn)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/api/v1/loyalty",
    responses(
        (status = NO_CONTENT, description = "Success")
    ),
    params(
        ("X-User-Name", Header, description = "Имя пользователя")
    ),
)]
pub async fn put_loyalty(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");

    let counter = diesel::insert_into(loyalty::table)
        .values(&Loyalty::new(username.to_owned()))
        .on_conflict(loyalty::username)
        .do_update()
        .set(loyalty::reservation_count.eq(loyalty::reservation_count + 1))
        .returning(loyalty::reservation_count)
        .get_result(conn)
        .map_err(|e| match e {
            DieselError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    if counter == 10 || counter == 20 {
        let (status, discount) = Loyalty::loyalty_from_counter(counter);

        diesel::update(loyalty::table)
            .filter(loyalty::username.eq(username))
            .set((loyalty::status.eq(status), loyalty::discount.eq(discount)))
            .execute(conn)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::NO_CONTENT)
}
