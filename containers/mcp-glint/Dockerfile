# syntax=docker/dockerfile:1
FROM rust:1.89-bookworm AS builder

WORKDIR /src
COPY glint /src/glint
RUN cd /src/glint && cargo build --release --locked

FROM debian:bookworm-slim AS glint-bin

ARG GLINT_VERSION=0.1.0
ARG BINS_RELEASE_VERSION=0.1.21
ARG TARGETARCH=amd64

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && curl -fsSL -o /usr/local/bin/glint \
       "https://github.com/envoyproxy/toolshed/releases/download/bins-v${BINS_RELEASE_VERSION}/glint-${GLINT_VERSION}-${TARGETARCH}" \
    && chmod +x /usr/local/bin/glint

FROM debian:bookworm-slim

ENV RUST_BACKTRACE=1

COPY --from=glint-bin /usr/local/bin/glint /usr/local/bin/glint
COPY --from=builder /src/glint/target/release/botwork-mcp-glint /usr/local/bin/botwork-mcp-glint

EXPOSE 8000
ENTRYPOINT ["botwork-mcp-glint"]
