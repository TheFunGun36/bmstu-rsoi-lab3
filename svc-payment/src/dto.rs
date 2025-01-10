use std::fmt::Display;

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct PaymentRequest {
    pub status: PaymentStatus,
    pub price: i32,
}

#[derive(Serialize, Queryable, Selectable, Insertable, ToSchema)]
#[diesel(table_name = crate::schema::payment)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub payment_uid: Uuid,
    pub status: String,
    pub price: i32,
}

impl From<PaymentRequest> for Payment {
    fn from(value: PaymentRequest) -> Self {
        Self {
            payment_uid: Uuid::new_v4(),
            status: value.status.to_string(),
            price: value.price,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Paid,
    Canceled,
}

impl Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paid => f.write_str("PAID"),
            Self::Canceled => f.write_str("CANCELED"),
        }
    }
}
