FROM rust:latest AS build
EXPOSE 8070

RUN mkdir /app
WORKDIR /app
COPY . .

RUN cargo build --release

ENTRYPOINT ["/bin/sh", "-c", "./target/release/bmstu-rsoi-lab2-payment"]

