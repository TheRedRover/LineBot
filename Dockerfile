FROM rust:latest
WORKDIR /app
COPY . .
RUN cargo build --release
EXPOSE 80
CMD ["target/release/queue-tg-bot"]
