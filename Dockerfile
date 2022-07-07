# Stage 1: Build frontend
FROM node:18.4.0 AS js-build

WORKDIR /usr/build/frontend
COPY ./frontend .

RUN yarn install --immutable --immutable-cache --check-cache
RUN yarn build

# Stage 2: Build backend
FROM rust:1.62.0 AS rust-build

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/build/backend
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.lock ./

RUN cargo build --release --locked --target x86_64-unknown-linux-musl
RUN strip target/x86_64-unknown-linux-musl/release/ws-storage-api

# Stage 3: Build runtime image
FROM scratch AS runtime

WORKDIR /app
COPY --from=js-build /usr/build/frontend/dist ./public
COPY --from=rust-build /usr/build/backend/target/x86_64-unknown-linux-musl/release/ws-storage-api .

EXPOSE 3030
CMD ["/app/ws-storage-api"]
