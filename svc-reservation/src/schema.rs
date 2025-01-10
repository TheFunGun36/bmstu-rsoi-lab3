// @generated automatically by Diesel CLI.

diesel::table! {
    hotels (id) {
        id -> Int4,
        hotel_uid -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 80]
        country -> Varchar,
        #[max_length = 80]
        city -> Varchar,
        #[max_length = 255]
        address -> Varchar,
        stars -> Nullable<Int4>,
        price -> Int4,
    }
}

diesel::table! {
    reservation (id) {
        id -> Int4,
        reservation_uid -> Uuid,
        #[max_length = 80]
        username -> Varchar,
        payment_uid -> Uuid,
        hotel_id -> Nullable<Int4>,
        #[max_length = 20]
        status -> Varchar,
        start_date -> Nullable<Timestamptz>,
        end_date -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(reservation -> hotels (hotel_id));

diesel::allow_tables_to_appear_in_same_query!(
    hotels,
    reservation,
);
