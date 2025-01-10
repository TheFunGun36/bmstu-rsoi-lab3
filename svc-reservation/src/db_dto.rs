use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::hotels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Hotel {
    pub hotel_uid: Uuid,
    pub name: String,
    pub country: String,
    pub city: String,
    pub address: String,
    pub stars: Option<i32>,
    pub price: i32,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::reservation)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Reservation {
    pub reservation_uid: Uuid,
    pub username: String,
    pub payment_uid: Uuid,
    pub hotel_id: Option<i32>,
    pub status: String,
    pub start_date: Option<chrono::DateTime<chrono::Local>>,
    pub end_date: Option<chrono::DateTime<chrono::Local>>,
}
