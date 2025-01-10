// @generated automatically by Diesel CLI.

diesel::table! {
    payment (id) {
        id -> Int4,
        payment_uid -> Uuid,
        #[max_length = 20]
        status -> Varchar,
        price -> Int4,
    }
}
