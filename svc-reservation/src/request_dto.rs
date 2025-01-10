use chrono::DateTime;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{db_dto, response_dto::ReservationStatus};

#[derive(Deserialize)]
pub struct Pagination {
    pub page: usize,
    pub size: usize,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationRequest {
    pub hotel_uid: Uuid,
    pub payment_uid: Uuid,
    pub start_date: Option<DateTime<chrono::Local>>,
    pub end_date: Option<DateTime<chrono::Local>>,
}

impl ReservationRequest {
    pub fn into_db_dto(self, username: String, hotel_id: Option<i32>) -> db_dto::Reservation {
        db_dto::Reservation {
            reservation_uid: Uuid::new_v4(),
            username,
            payment_uid: self.payment_uid,
            hotel_id,
            status: ReservationStatus::Paid.to_string(),
            start_date: self.start_date,
            end_date: self.end_date,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationPath {
    pub reservation_uid: Uuid,
}
