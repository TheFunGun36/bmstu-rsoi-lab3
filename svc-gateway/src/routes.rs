use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{Duration, NaiveTime, Utc};
use uuid::Uuid;

use crate::{
    dto::*, AppState, Message, RequestReturnValue, LOYALTY_ENDPOINT, PAYMENT_ENDPOINT,
    RESERVATION_ENDPOINT,
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

    let client = reqwest::Client::new();
    let loyalty = client
        .get(format!("{LOYALTY_ENDPOINT}/api/v1/loyalty"))
        .header("X-User-Name", username)
        .send()
        .await;
    let loyalty = match loyalty {
        Err(e) => {
            log::warn!("Failed to issue request to reservation service: {e}");
            None
        }
        Ok(l) if l.status().is_client_error() => return Err(l.status()),
        Ok(l) => LoyaltyInfoResponse::try_from_json(l).await,
    };

    let reservations = client
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
                .await;
            let payment_info = match payment_info {
                Err(e) => {
                    log::warn!("Failed to issue request to reservation service: {e}");
                    None
                }
                Ok(p) => PaymentInfo::try_from_json(p).await,
            };

            ReservationResponse::from_svc_responses(el, payment_info)
        })
        .collect::<Vec<_>>();

    let reservations = futures::future::join_all(reservations).await;

    Ok((
        StatusCode::OK,
        Json(UserInfoResponse {
            reservations,
            loyalty: LoyaltyInfoResponse::from_opt(loyalty),
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
                .await;
            let payment_info = match payment_info {
                Err(e) => {
                    log::warn!("Failed to issue request to payment service: {e}");
                    None
                }
                Ok(p) => match p.json::<PaymentInfo>().await {
                    Err(e) => {
                        log::warn!("Failed to parse payment service response: {e}");
                        None
                    }
                    Ok(p) => Some(p),
                },
            };

            ReservationResponse::from_svc_responses(el, payment_info)
        })
        .collect::<Vec<_>>();

    let resp = futures::future::join_all(resp).await;

    Ok(Json(resp))
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
) -> Result<impl IntoResponse, impl IntoResponse> {
    let username = headers
        .get("X-User-Name")
        .ok_or(StatusCode::BAD_REQUEST.into_response())?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;

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
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        })?
        .error_for_status()
        .map_err(|e| {
            e.status()
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                .into_response()
        })?
        .json::<HotelResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
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
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    message: "Loyalty Service unavailable".to_owned(),
                }),
            )
                .into_response()
        })?;
    let loyalty = match loyalty.status() {
        StatusCode::NOT_FOUND => LoyaltyInfoResponse {
            status: Some(LoyaltyStatus::Bronze),
            discount: Some(5),
            reservation_count: Some(1),
        },
        StatusCode::OK => LoyaltyInfoResponse::from_json(loyalty)
            .await
            .map_err(StatusCode::into_response)?,
        status => {
            log::error!("unexpected loyalty service response: {status}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };

    let cost = cost - (cost * loyalty.discount.unwrap() / 100);

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
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        })?
        .error_for_status()
        .map_err(|e| {
            e.status()
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                .into_response()
        })?
        .json::<PaymentInfoServiceResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse payment service response: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
    log::debug!("Successfully created payment record");

    // 5) запись в loyalty
    let l = client
        .put(format!("{}/api/v1/loyalty", LOYALTY_ENDPOINT))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to loyalty service: {e}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        })
        .and_then(|r| {
            r.error_for_status().map_err(|e| {
                e.status()
                    .map(StatusCode::into_response)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            })
        });
    log::debug!("Successfully created loyalty record");
    match l {
        // 6.1) Сервис доступен, завершаем операцию
        Ok(_) => {
            // 6.2) запись в reservation
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
                    (StatusCode::SERVICE_UNAVAILABLE,).into_response()
                })?
                .error_for_status()
                .map_err(|e| {
                    e.status()
                        .map(StatusCode::into_response)
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response())
                })?
                .json::<PostReservationServiceResponse>()
                .await
                .map_err(|e| {
                    log::error!("Failed to parse reservation service response: {e}");
                    (StatusCode::INTERNAL_SERVER_ERROR,).into_response()
                })?;
            log::debug!("Successfully created reservation record");

            Ok(Json(CreateReservationResponse {
                reservation_uid: reservation.reservation_uid,
                hotel_uid: reservation.hotel_uid,
                start_date: reservation.start_date.naive_utc().date(),
                end_date: reservation.end_date.naive_utc().date(),
                discount: loyalty.discount.unwrap(),
                status: reservation.status,
                payment: PaymentInfo {
                    status: payment.status,
                    price: payment.price,
                },
            }))
        }
        // 7.1) Ошибка при обращении в loyalty сервис, откатываем payment
        Err(_) => {
            log::warn!("loyalty service unavailable, roll back payment");
            client
                .delete(format!(
                    "{}/api/v1/payment/{}",
                    PAYMENT_ENDPOINT, payment.payment_uid
                ))
                .send()
                .await
                .map_err(|e| {
                    log::error!("Failed to issue request to payment service: {e}");
                    StatusCode::SERVICE_UNAVAILABLE.into_response()
                })?
                .error_for_status()
                .map_err(|e| {
                    e.status()
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                        .into_response()
                })?
                .json::<PaymentInfoServiceResponse>()
                .await
                .map_err(|e| {
                    log::error!("Failed to parse payment service response: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                })?;
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    message: "Loyalty Service unavailable".to_owned(),
                }),
            )
                .into_response())
        }
    }
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
        .await;
    let payment = match payment {
        Err(e) => {
            log::warn!("Failed to issue request to payment service: {e}");
            None
        }
        Ok(p) => PaymentInfo::try_from_json(p).await,
    };

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
    State(state): State<AppState>,
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

    let loyalty_resp = client
        .delete(format!("{}/api/v1/loyalty", LOYALTY_ENDPOINT))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to loyalty service: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })
        .and_then(|s| {
            s.error_for_status()
                .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        });

    if let Err(e) = loyalty_resp {
        log::debug!("Loyalty service unavailable ({e}), request is being put into send queue");
        let username = username.to_owned();
        let resend_lambda = Box::new(move || -> RequestReturnValue {
            let username = username.clone();
            Box::pin(async move {
                reqwest::Client::new()
                    .delete(format!("{}/api/v1/loyalty", LOYALTY_ENDPOINT))
                    .header("X-User-Name", username)
                    .send()
                    .await
                    .map_err(|e| {
                        log::error!("Failed to issue request to loyalty service: {e}");
                        StatusCode::SERVICE_UNAVAILABLE
                    })
                    .and_then(|s| {
                        s.error_for_status()
                            .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
                    })
                    .map(|_| ())
            })
        });
        state
            .msg_chan
            .send(Message {
                timeout: Utc::now() + Duration::seconds(10),
                request: resend_lambda,
            })
            .await
            .expect("Failed to add message to the queue");
    }

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
pub async fn get_loyalty(headers: HeaderMap) -> Result<impl IntoResponse, impl IntoResponse> {
    let username = headers
        .get("X-User-Name")
        .ok_or(ErrorResponse::resp_from_status(StatusCode::BAD_REQUEST))?
        .to_str()
        .map_err(|_| ErrorResponse::resp_from_status(StatusCode::BAD_REQUEST))?;

    let resp = reqwest::Client::new()
        .get(format!("{LOYALTY_ENDPOINT}/api/v1/loyalty"))
        .header("X-User-Name", username)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to issue request to reservation service: {e}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    message: "Loyalty Service unavailable".to_owned(),
                }),
            )
        })?
        .error_for_status()
        .map_err(|e| {
            ErrorResponse::resp_from_status(e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        })?
        .json::<LoyaltyInfoResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse reservation service response: {e}");
            ErrorResponse::resp_from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    Ok::<_, (StatusCode, Json<ErrorResponse>)>(Json(resp))
}
