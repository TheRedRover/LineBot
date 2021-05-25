FROM rust:latest AS build
WORKDIR /app
COPY . /app/
RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update && apt-get install -y libssl-dev libpq-dev && rm -rf /var/lib/apt/lists/*
COPY --from=build /app /app
EXPOSE 80
CMD ["/app/target/release/queue-tg-bot"]
