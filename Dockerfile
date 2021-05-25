FROM rust:latest AS build
WORKDIR /app
COPY . /app/
RUN cargo build --release
EXPOSE 80
CMD ["target/release/queue-tg-bot"]
