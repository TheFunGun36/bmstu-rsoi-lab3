use std::{fmt::Display, str::FromStr};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::db_dto;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Hotel {
    pub hotel_uid: Uuid,
    pub name: String,
    pub country: String,
    pub city: String,
    pub address: String,
    pub stars: Option<i32>,
    pub price: i32,
}

impl From<crate::db_dto::Hotel> for Hotel {
    fn from(value: crate::db_dto::Hotel) -> Self {
        Self {
            hotel_uid: value.hotel_uid,
            name: value.name,
            country: value.country,
            city: value.city,
            address: value.address,
            stars: value.stars,
            price: value.price,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HotelList {
    pub page: usize,
    pub page_size: usize,
    pub total_elements: usize,
    pub items: Vec<Hotel>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HotelShort {
    pub hotel_uid: Uuid,
    pub name: String,
    pub full_address: String,
    pub stars: Option<i32>,
}

impl From<db_dto::Hotel> for HotelShort {
    fn from(value: db_dto::Hotel) -> Self {
        Self {
            hotel_uid: value.hotel_uid,
            name: value.name,
            full_address: format!("{}, {}, {}", value.country, value.city, value.address),
            stars: value.stars,
        }
    }
}

impl From<Hotel> for HotelShort {
    fn from(value: Hotel) -> Self {
        Self {
            hotel_uid: value.hotel_uid,
            name: value.name,
            full_address: format!("{}, {}, {}", value.country, value.city, value.address),
            stars: value.stars,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Reservation {
    pub reservation_uid: Uuid,
    pub hotel_uid: Uuid,
    pub payment_uid: Uuid,
    pub status: ReservationStatus,
    pub start_date: Option<DateTime<chrono::Local>>,
    pub end_date: Option<DateTime<chrono::Local>>,
}

impl Reservation {
    pub fn from_db_dto(value: db_dto::Reservation, hotel_uid: Uuid) -> Self {
        Self {
            reservation_uid: value.reservation_uid,
            payment_uid: value.payment_uid,
            hotel_uid,
            status: ReservationStatus::from_str(value.status.as_str()).unwrap(),
            start_date: value.start_date,
            end_date: value.end_date,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationWithHotel {
    pub reservation_uid: Uuid,
    pub payment_uid: Uuid,
    pub status: ReservationStatus,
    pub start_date: Option<DateTime<chrono::Local>>,
    pub end_date: Option<DateTime<chrono::Local>>,
    pub hotel: HotelShort,
}

impl ReservationWithHotel {
    pub fn from_db_dto(value: db_dto::Reservation, hotel: db_dto::Hotel) -> Self {
        Self {
            reservation_uid: value.reservation_uid,
            payment_uid: value.payment_uid,
            hotel: hotel.into(),
            status: ReservationStatus::from_str(value.status.as_str()).unwrap(),
            start_date: value.start_date,
            end_date: value.end_date,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReservationStatus {
    Paid,
    Canceled,
}

impl Display for ReservationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ReservationStatus::Paid => "PAID",
            ReservationStatus::Canceled => "CANCELED",
        };

        f.write_str(s)
    }
}

impl FromStr for ReservationStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PAID" => Ok(Self::Paid),
            "CANCELED" => Ok(Self::Canceled),
            _ => Err(()),
        }
    }
}
