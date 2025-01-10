use axum::{
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::NaiveTime;
use uuid::Uuid;

use crate::{dto::*, LOYALTY_ENDPOINT, PAYMENT_ENDPOINT, RESERVATION_ENDPOINT};

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
            body = PaginationResponse,
            content_type = "application/json",
        ),
    ),
    params(
        ("page", Query, description="Количество страниц"),
        ("size", Query, description="Количество элементов страницы")
    ),
)]
pub async fn get_hotels(
    Query(pagination): Query<PaginationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{RESERVATION_ENDPOINT}/api/v1/hotels"))
        .query(&pagination)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .json::<PaginationResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/api/v1/me",
    responses(
        (
            status = OK,
            description = "Полная информация о пользователе",
            body = UserInfoResponse,
            content_type = "application/json",
        ),
    ),
    params(
        ("X-User-Name", Header, description="Имя пользователя"),
    ),
)]
pub async fn get_me(headers: HeaderMap) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let loyalty = reqwest::Client::new()
        .get(format!("{LOYALTY_ENDPOINT}/api/v1/loyalty"))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?
        .json::<LoyaltyInfoResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let reservations = reqwest::Client::new()
        .get(format!("{RESERVATION_ENDPOINT}/api/v1/reservations"))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?
        .json::<Vec<ReservationServiceResponse>>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let reservations = reservations
        .into_iter()
        .map(|el| async {
            let payment_info = reqwest::Client::new()
                .get(format!(
                    "{}/api/v1/payment/{}",
                    PAYMENT_ENDPOINT, el.payment_uid
                ))
                .send()
                .await
                .map_err(|e| {
                    log::error!("Failed to issue request to reservation service: {e}");
                    StatusCode::SERVICE_UNAVAILABLE
                })?
                .json::<PaymentInfo>()
                .await
                .map_err(|e| {
                    log::error!("Failed to parse reservation service response: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            Ok(ReservationResponse::from_svc_responses(el, payment_info))
        })
        .collect::<Vec<_>>();

    let reservations: Result<_, StatusCode> = futures::future::try_join_all(reservations).await;

    Ok((
        StatusCode::OK,
        Json(UserInfoResponse {
            reservations: reservations?,
            loyalty,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/reservations",
    responses(
        (
            status = OK,
            description = "Информация по всем билетам",
            body = Vec<ReservationResponse>,
            content_type = "application/json",
        ),
    ),
    params(
        ("X-User-Name", Header, description = "Имя пользователя")
    ),
)]
pub async fn get_reservations(headers: HeaderMap) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let resp = reqwest::Client::new()
        .get(format!("{RESERVATION_ENDPOINT}/api/v1/reservations"))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .json::<Vec<ReservationServiceResponse>>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let resp = resp
        .into_iter()
        .map(|el| async {
            let payment_info = reqwest::Client::new()
                .get(format!(
                    "{}/api/v1/payment/{}",
                    PAYMENT_ENDPOINT, el.payment_uid
                ))
                .send()
                .await
                .map_err(|e| {
                    log::error!("Failed to issue request to reservation service: {e}");
                    StatusCode::SERVICE_UNAVAILABLE
                })?
                .json::<PaymentInfo>()
                .await
                .map_err(|e| {
                    log::error!("Failed to parse reservation service response: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            Ok(ReservationResponse::from_svc_responses(el, payment_info))
        })
        .collect::<Vec<_>>();

    let resp: Result<_, StatusCode> = futures::future::try_join_all(resp).await;

    Ok(Json(resp?))
}

#[utoipa::path(
    post,
    path = "/api/v1/reservations",
    responses(
        (
            status = OK,
            description = "Информация о бронировании",
            body = CreateReservationResponse,
            content_type = "application/json",
        ),
    ),
    params(
        ("X-User-Name", Header, description = "Имя пользователя")
    ),
)]
pub async fn post_reservation(
    headers: HeaderMap,
    Json(req): Json<CreateReservationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let client = reqwest::Client::new();
    // 1) запросить отель
    let hotel = client
        .get(format!(
            "{}/api/v1/hotel/{}",
            RESERVATION_ENDPOINT, req.hotel_uid
        ))
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?
        .json::<HotelResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // 2) рассчитать по нему стоимость (end_date - start_date)
    let cost = ((req.end_date - req.start_date).num_days() * hotel.price as i64) as i32;

    // 3) рассчитать скидку
    let loyalty = client
        .get(format!("{}/api/v1/loyalty", LOYALTY_ENDPOINT))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to loyalty service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?;
    let loyalty = match loyalty.status() {
        StatusCode::NOT_FOUND => LoyaltyInfoResponse {
            status: LoyaltyStatus::Bronze,
            discount: 5,
            reservation_count: 1,
        },
        StatusCode::OK => loyalty.json::<LoyaltyInfoResponse>().await.map_err(|e| {
            log::error!("Failed to parse loyalty service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?,
        unknown_status_code => return Err(unknown_status_code),
    };

    let cost = cost - (cost * loyalty.discount / 100);

    // 4) запись в payment
    let payment = client
        .post(format!("{}/api/v1/payment", PAYMENT_ENDPOINT))
        .json(&PaymentInfo {
            status: PaymentStatus::Paid,
            price: cost as i32,
        })
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to payment service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?
        .json::<PaymentInfoServiceResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse payment service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    log::debug!("Successfully created payment record");

    // 5) запись в loyalty
    client
        .put(format!("{}/api/v1/loyalty", LOYALTY_ENDPOINT))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to loyalty service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?;
    log::debug!("Successfully created loyalty record");

    // 6) запись в reservation
    let reservation = client
        .post(format!("{}/api/v1/reservations", RESERVATION_ENDPOINT))
        .header("X-User-Name", username)
        .json(&PostReservationServiceRequest {
            hotel_uid: req.hotel_uid,
            payment_uid: payment.payment_uid,
            start_date: req
                .start_date
                .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc()
                .into(),
            end_date: req
                .end_date
                .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc()
                .into(),
        })
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?
        .json::<PostReservationServiceResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    log::debug!("Successfully created reservation record");

    Ok(Json(CreateReservationResponse {
        reservation_uid: reservation.reservation_uid,
        hotel_uid: reservation.hotel_uid,
        start_date: reservation.start_date.naive_utc().date(),
        end_date: reservation.end_date.naive_utc().date(),
        discount: loyalty.discount,
        status: reservation.status,
        payment: PaymentInfo {
            status: payment.status,
            price: payment.price,
        },
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/reservations/{reservationUid}",
    responses(
        (
            status = OK,
            description = "Информация по одному бронированию",
            body = ReservationResponse,
            content_type = "application/json",
        ),
    ),
    params(
        ("X-User-Name", Header, description = "Имя пользователя"),
        ("reservationUid", Path, description = "Идентификатор запрашиваемой брони"),
    ),
)]
pub async fn get_reservation(
    Path(reservation_uid): Path<Uuid>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let client = reqwest::Client::new();
    let reservation = client
        .get(format!(
            "{RESERVATION_ENDPOINT}/api/v1/reservations/{reservation_uid}"
        ))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .json::<ReservationServiceResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let payment = client
        .get(format!(
            "{}/api/v1/payment/{}",
            PAYMENT_ENDPOINT, reservation.payment_uid
        ))
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .json::<PaymentInfo>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ReservationResponse::from_svc_responses(
        reservation,
        payment,
    )))
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
    Path(reservation_uid): Path<Uuid>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let client = reqwest::Client::new();
    let reservation = client
        .get(format!(
            "{}/api/v1/reservations/{}",
            RESERVATION_ENDPOINT, reservation_uid
        ))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?
        .json::<ReservationServiceResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    client
        .delete(format!(
            "{}/api/v1/reservations/{}",
            RESERVATION_ENDPOINT, reservation_uid
        ))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?;

    client
        .delete(format!(
            "{}/api/v1/payment/{}",
            PAYMENT_ENDPOINT, reservation.payment_uid
        ))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to payment service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?;

    client
        .delete(format!("{}/api/v1/loyalty", LOYALTY_ENDPOINT))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to loyalty service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/loyalty",
    responses(
        (status = OK, body = LoyaltyInfoResponse, description = "Данные о бонусном счёте")
    ),
    params(
        ("X-User-Name", Header, description="Имя пользователя, для которого будет заведена бронь")
    ),
)]
pub async fn get_loyalty(headers: HeaderMap) -> Result<impl IntoResponse, StatusCode> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let resp = reqwest::Client::new()
        .get(format!("{LOYALTY_ENDPOINT}/api/v1/loyalty"))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?
        .error_for_status()
        .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))?
        .json::<LoyaltyInfoResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok((StatusCode::OK, Json(resp)))
}
