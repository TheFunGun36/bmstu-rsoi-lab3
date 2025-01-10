use diesel::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::loyalty)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Loyalty {
    pub username: String,
    pub reservation_count: i32,
    pub status: String,
    pub discount: i32,
}

impl Loyalty {
    pub fn new(username: String) -> Self {
        Self {
            username,
            reservation_count: 1,
            status: "BRONZE".to_owned(),
            discount: 5,
        }
    }

    // returns status and discount
    pub fn loyalty_from_counter(counter: i32) -> (String, i32) {
        match counter {
            _ if counter >= 20 => ("GOLD".to_owned(), 10),
            _ if counter >= 10 => ("SILVER".to_owned(), 7),
            _ => ("BRONZE".to_owned(), 5),
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoyaltyResponse {
    pub status: String,
    pub discount: i32,
    pub reservation_count: i32,
}

impl From<Loyalty> for LoyaltyResponse {
    fn from(value: Loyalty) -> Self {
        Self {
            status: value.status,
            discount: value.discount,
            reservation_count: value.reservation_count,
        }
    }
}

