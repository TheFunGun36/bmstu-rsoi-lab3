// @generated automatically by Diesel CLI.

diesel::table! {
    loyalty (id) {
        id -> Int4,
        #[max_length = 80]
        username -> Varchar,
        reservation_count -> Int4,
        #[max_length = 80]
        status -> Varchar,
        discount -> Int4,
    }
}
