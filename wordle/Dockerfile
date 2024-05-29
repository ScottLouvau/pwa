ARG PROD_IMAGE=gcr.io/distroless/cc-debian12
# gcr.io/distroless/cc-debian12         [34.6 MB; default Rust build works]
# gcr.io/distroless/cc-debian12:debug   [38.6 MB; default build; adds console access]
# ubuntu:latest                         [69.8 MB; default Rust build works; vulnerabilities]

# Build Rust toolchain image
FROM rust:latest AS build-base
RUN apt update -y

# Experiments for Code Coverage and CPU Profiling in Docker
#RUN apt install -y linux-perf linux-base
#RUN cargo install flamegraph

# Build wordle-http
FROM build-base AS builder

# ... copy empty main to pre-build and cache dependencies
WORKDIR /usr/local/wordle

COPY ./data ./data

COPY lib-wordle/src ./lib-wordle/src
COPY lib-wordle/Cargo.toml ./lib-wordle/
COPY lib-wordle/Cargo.lock ./lib-wordle/

COPY bin-wordle-http/Cargo.toml ./bin-wordle-http/
COPY bin-wordle-http/Cargo.lock ./bin-wordle-http/
COPY stub-main.rs ./bin-wordle-http/src/main.rs

WORKDIR /usr/local/wordle/bin-wordle-http
RUN cargo build --release

# ... copy real code and (incremental) build
RUN rm -f ./src/main.rs

COPY bin-wordle-http/index.html ./
COPY bin-wordle-http/src/ ./src/

RUN touch ./src/main.rs
RUN cargo build --release

# Build production image to run app.
FROM ${PROD_IMAGE}
WORKDIR /app
COPY ./data ./data/
COPY --from=builder --chown=base:nonroot /usr/local/wordle/bin-wordle-http/target/release/wordle-http ./

USER nonroot

EXPOSE 8080
ENTRYPOINT ["./wordle-http"]

# See: https://groups.google.com/g/distroless-users/c/lqtS_c0tCaE?pli=1
# NOTE: Error if using 'CMD' instead of 'ENTRYPOINT' on :debug distroless image.
#  Executable exits immediately with:
#    ./wordle-http: line 1: ELF�n@�X�@8: not found
#    ./wordle-http: line 2: syntax error: unexpected ")"

# Supposedly, maybe a difference between direct busybox and a shell (debug vs. non-debug image)
# But running in the debug image interactively, I can run the exe fine.