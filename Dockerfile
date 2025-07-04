FROM rustlang/rust:nightly as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/thirdweb-engine /usr/local/bin/
COPY --from=builder /app/server/configuration /configuration
EXPOSE 3069
CMD ["thirdweb-engine"]
