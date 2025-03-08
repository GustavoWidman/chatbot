# syntax=docker/dockerfile:1.3-labs

FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev sqlite-static openssl-dev openssl-libs-static pkgconf git libpq-dev
ENV SYSROOT=/dummy

# ARG VARIABLE_1
# ARG VARIABLE_2
# ...

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

# dump the environment to a .env file (dotenvy_macro will read this file)
# RUN echo "VARIABLE_1=$VARIABLE_1" > .env
# RUN echo "VARIABLE_2=$VARIABLE_2" >> .env
# ...

RUN --mount=type=cache,target=/target/ \
	--mount=type=cache,target=/usr/local/cargo/git/db \
	--mount=type=cache,target=/usr/local/cargo/registry/ \
	cargo build --release && \
	cp /target/release/chatbot /app

FROM scratch
COPY --from=builder /app /app
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
CMD ["/app"]