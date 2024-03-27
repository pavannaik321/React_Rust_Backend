# Build stage
FROM rust:1.69-buster as builder

WORKDIR /app

# Accept the build argument
ARG DATABASE_URL

ENV DATABASE_URL=$DATABASE_URL

ENV GIT_SSL_NO_VERIFY=1

ENV GIT_SSL_CAINFO=/path/to/ca-certificates.crt



# Install any additional dependencies needed for your application
RUN apt-get update && apt-get install -y build-essential

COPY . .

# Compile the Rust application
RUN cargo build --release

# Production stage
FROM debian:buster-slim

WORKDIR /usr/local/bin

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/backend .

CMD ["./backend"]
