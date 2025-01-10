-- Sets up a trigger for the given table to automatically set a column called
-- `updated_at` whenever the row is modified (unless `updated_at` was included
-- in the modified columns)
--
-- # Example
--
-- ```sql
-- CREATE TABLE users (id SERIAL PRIMARY KEY, updated_at TIMESTAMP NOT NULL DEFAULT NOW());
--
-- SELECT diesel_manage_updated_at('users');
-- ```
CREATE OR REPLACE FUNCTION diesel_manage_updated_at(_tbl regclass) RETURNS VOID AS $$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s
                    FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at()', _tbl);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION diesel_set_updated_at() RETURNS trigger AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE IF NOT EXISTS hotels
(
    id        SERIAL PRIMARY KEY,
    hotel_uid UUID NOT NULL UNIQUE,
    name      VARCHAR(255) NOT NULL,
    country   VARCHAR(80)  NOT NULL,
    city      VARCHAR(80)  NOT NULL,
    address   VARCHAR(255) NOT NULL,
    stars     INT,
    price     INT          NOT NULL
);

CREATE TABLE IF NOT EXISTS reservation
(
    id              SERIAL PRIMARY KEY,
    reservation_uid UUID UNIQUE NOT NULL,
    username        VARCHAR(80) NOT NULL,
    payment_uid     UUID NOT NULL,
    hotel_id        INT REFERENCES hotels (id),
    status          VARCHAR(20) NOT NULL
        CHECK (status IN ('PAID', 'CANCELED')),
    start_date      TIMESTAMP WITH TIME ZONE,
    end_date        TIMESTAMP WITH TIME ZONE
);

INSERT INTO hotels(
	hotel_uid,
	name,
	country,
	city,
	address,
	stars,
	price
) VALUES(
	'049161bb-badd-4fa8-9d90-87c9a82b0668',
	'Ararat Park Hyatt Moscow',
	'Россия',
	'Москва',
	'Неглинная ул., 4',
	5,
	10000
);

