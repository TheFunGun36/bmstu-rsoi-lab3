use std::fmt::Display;

use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationRequest {
    page: usize,
    size: usize,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationResponse {
    page: usize,
    page_size: usize,
    total_elements: usize,
    items: Vec<HotelResponse>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HotelResponse {
    pub hotel_uid: Uuid,
    pub name: String,
    pub country: String,
    pub city: String,
    pub address: String,
    pub stars: i32,
    pub price: i32,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HotelInfo {
    hotel_uid: Uuid,
    name: String,
    full_address: String,
    stars: i32,
}

#[derive(Serialize, ToSchema)]
pub struct UserInfoResponse {
    pub reservations: Vec<ReservationResponse>,
    pub loyalty: LoyaltyInfoResponse,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationResponse {
    reservation_uid: Uuid,
    hotel: HotelInfo,
    start_date: NaiveDate,
    end_date: NaiveDate,
    status: PaymentStatus,
    payment: PaymentInfo,
}

impl ReservationResponse {
    pub fn from_svc_responses(res: ReservationServiceResponse, payment: PaymentInfo) -> Self {
        Self {
            reservation_uid: res.reservation_uid,
            hotel: res.hotel,
            start_date: res.start_date.date_naive(),
            end_date: res.end_date.date_naive(),
            status: res.status,
            payment,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationServiceResponse {
    pub reservation_uid: Uuid,
    pub hotel: HotelInfo,
    pub start_date: DateTime<chrono::Local>,
    pub end_date: DateTime<chrono::Local>,
    pub status: PaymentStatus,
    pub payment_uid: Uuid,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInfo {
    pub status: PaymentStatus,
    pub price: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInfoServiceResponse {
    pub payment_uid: Uuid,
    pub status: PaymentStatus,
    pub price: i32,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateReservationRequest {
    pub hotel_uid: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateReservationResponse {
    pub reservation_uid: Uuid,
    pub hotel_uid: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub discount: i32,
    pub status: PaymentStatus,
    pub payment: PaymentInfo,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PostReservationServiceRequest {
    pub hotel_uid: Uuid,
    pub payment_uid: Uuid,
    pub start_date: DateTime<chrono::Local>,
    pub end_date: DateTime<chrono::Local>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PostReservationServiceResponse {
    pub reservation_uid: Uuid,
    pub hotel_uid: Uuid,
    pub payment_uid: Uuid,
    pub start_date: DateTime<chrono::Local>,
    pub end_date: DateTime<chrono::Local>,
    pub status: PaymentStatus,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoyaltyInfoResponse {
    pub status: LoyaltyStatus,
    pub discount: i32,
    pub reservation_count: i32,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Paid,
    Canceled,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LoyaltyStatus {
    Bronze,
    Silver,
    Gold,
}

impl Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paid => f.write_str("PAID"),
            Self::Canceled => f.write_str("CANCELED"),
        }
    }
}
