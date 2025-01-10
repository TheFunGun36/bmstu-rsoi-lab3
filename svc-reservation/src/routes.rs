use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use diesel::{prelude::*, result::Error as DieselError};
use uuid::Uuid;

use crate::{
    db_dto,
    diesel_paginate::*,
    request_dto, response_dto,
    schema::{hotels, reservation},
    AppState,
};

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
    path = "/api/v1/hotels",
    responses(
        (
            status = OK,
            description = "Список отелей",
            body = response_dto::HotelList,
            content_type = "application/json",
        ),
    ),
    params(
        ("page", Query, description="Количество страниц"),
        ("size", Query, description="Количество элементов страницы")
    ),
)]
pub async fn get_hotels(
    State(state): State<AppState>,
    Query(pagination): Query<request_dto::Pagination>,
) -> impl IntoResponse {
    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");
    let res = hotels::table
        .order(hotels::name)
        .select(db_dto::Hotel::as_select())
        .paginate(pagination.page as i64)
        .per_page(pagination.size as i64)
        .load_and_count_pages(conn);

    match res {
        Ok((hotels, count)) => (
            StatusCode::OK,
            Json(response_dto::HotelList {
                page: pagination.page,
                page_size: pagination.size,
                total_elements: count as usize,
                items: hotels.into_iter().map(response_dto::Hotel::from).collect(),
            }),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/hotel/{hotelId}",
    responses(
        (
            status = OK,
            description = "Список отелей",
            body = response_dto::Hotel,
            content_type = "application/json",
        ),
    ),
    params(
        ("hotelid", Path, description="ID отеля"),
    ),
)]
pub async fn get_hotel(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");
    let res = hotels::table
        .filter(hotels::hotel_uid.eq(uid))
        .select(db_dto::Hotel::as_select())
        .get_result(conn)
        .map_err(|e| match e {
            DieselError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(Json(response_dto::Hotel::from(res)))
}

#[utoipa::path(
    get,
    path = "/api/v1/reservations",
    responses(
        (
            status = OK,
            description = "Информация по всем билетам",
            body = Vec<response_dto::ReservationWithHotel>,
            content_type = "application/json",
        ),
    ),
    params(
        ("X-User-Name", Header, description = "Имя пользователя")
    ),
)]
pub async fn get_reservations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_name = match headers.get("X-User-Name").and_then(|v| v.to_str().ok()) {
        Some(v) => v,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");
    let res = reservation::table
        .filter(reservation::username.eq(user_name))
        .inner_join(hotels::table)
        .select((db_dto::Reservation::as_select(), db_dto::Hotel::as_select()))
        .load(conn);

    match res {
        Ok(r) => (
            StatusCode::OK,
            Json(
                r.into_iter()
                    .map(|(reservation, hotel)| {
                        response_dto::ReservationWithHotel::from_db_dto(reservation, hotel)
                    })
                    .collect::<Vec<_>>(),
            ),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/reservations/{reservationUid}",
    responses(
        (
            status = OK,
            description = "Информация по всем бронированию",
            body = response_dto::ReservationWithHotel,
            content_type = "application/json",
        ),
    ),
    params(
        ("X-User-Name", Header, description = "Имя пользователя"),
        ("reservationUid", Path, description = "Идентификатор запрашиваемой брони"),
    ),
)]
pub async fn get_reservation(
    State(state): State<AppState>,
    Path(path): Path<request_dto::ReservationPath>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");
    let (reservation, hotel) = reservation::table
        .filter(reservation::username.eq(username))
        .filter(reservation::reservation_uid.eq(path.reservation_uid))
        .inner_join(hotels::table)
        .select((db_dto::Reservation::as_select(), db_dto::Hotel::as_select()))
        .get_result(conn)
        .map_err(|e| match e {
            DieselError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok((
        StatusCode::OK,
        Json(response_dto::ReservationWithHotel::from_db_dto(
            reservation,
            hotel,
        )),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/reservations/{reservationUid}",
    responses(
        (
            status = NO_CONTENT,
            description = "Бронирование отменено",
            content_type = "application/json",
        ),
    ),
    params(
        ("X-User-Name", Header, description = "Имя пользователя"),
        ("reservationUid", Path, description = "Идентификатор запрашиваемой брони"),
    ),
)]
pub async fn delete_reservation(
    State(state): State<AppState>,
    Path(path): Path<request_dto::ReservationPath>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");
    diesel::update(reservation::table)
        .filter(reservation::username.eq(username))
        .filter(reservation::reservation_uid.eq(path.reservation_uid))
        .set(reservation::status.eq(response_dto::ReservationStatus::Canceled.to_string()))
        .execute(conn)
        .map_err(|e| match e {
            DieselError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/reservations",
    responses(
        (status = CREATED, body = response_dto::Reservation, description = "Success")
    ),
    params(
        ("X-User-Name", Header, description="Имя пользователя, для которого будет заведена бронь")
    ),
)]
pub async fn post_reservation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(reservation): Json<request_dto::ReservationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let conn = &mut PgConnection::establish(state.database_url.as_str())
        .expect("Failed to establish connection to database");

    let hotel_uid = reservation.hotel_uid;

    let id = hotels::table
        .filter(hotels::hotel_uid.eq(hotel_uid))
        .select(hotels::id)
        .get_result(conn)
        .map_err(|e| match e {
            DieselError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let post_reservation = reservation.into_db_dto(username.to_owned(), Some(id));
    let created_reservation = diesel::insert_into(reservation::table)
        .values(&post_reservation)
        .returning(db_dto::Reservation::as_returning())
        .get_result(conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response_reservation =
        response_dto::Reservation::from_db_dto(created_reservation, hotel_uid);

    Ok((StatusCode::CREATED, Json(response_reservation)))
}

//
// #[utoipa::path(
//     patch,
//     path = "/api/v1/persons/{personId}",
//     responses(
//         (status = OK, description = "Success")
//     )
// )]
// pub async fn patch_person(
//     State(state): State<AppState>,
//     Path(person_id): Path<i32>,
//     Json(person): Json<PersonPatchRequest>,
// ) -> impl IntoResponse {
//     let conn = &mut establish_connection(state.database_url.as_str());
//     let res = diesel::update(person::table)
//         .filter(person::id.eq(person_id))
//         .set(person)
//         .returning(PersonResponse::as_returning())
//         .get_result(conn);
//
//     match res {
//         Ok(updated_person) => (StatusCode::OK, Json(updated_person)).into_response(),
//         Err(_) => StatusCode::NOT_FOUND.into_response(),
//     }
// }
//
// #[utoipa::path(
//     delete,
//     path = "/api/v1/persons/{personId}",
//     responses(
//         (status = NO_CONTENT, description = "Success")
//     )
// )]
// pub async fn delete_person(
//     Path(person_id): Path<i32>,
//     State(state): State<AppState>,
// ) -> impl IntoResponse {
//     let conn = &mut establish_connection(state.database_url.as_str());
//
//     let res = diesel::delete(person::table)
//         .filter(person::id.eq(person_id))
//         .execute(conn);
//
//     match res {
//         Ok(_) => StatusCode::NO_CONTENT,
//         Err(_) => StatusCode::NOT_FOUND,
//     }
// }
