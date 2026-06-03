ARG BINARY_SOURCE=source

FROM rust:1.89-bookworm AS builder

WORKDIR /src/echo
COPY echo /src/echo
RUN cargo build --release --locked

FROM scratch AS source
COPY --from=builder /src/echo/target/release/botwork-mcp-echo /botwork-mcp-echo

FROM scratch AS prebuilt
COPY .ci-prebuilt/echo/botwork-mcp-echo /botwork-mcp-echo

FROM ${BINARY_SOURCE} AS binary-source

FROM debian:bookworm-slim

ARG BINARY_SOURCE=source
ENV RUST_BACKTRACE=1

COPY --from=binary-source /botwork-mcp-echo /usr/local/bin/botwork-mcp-echo

EXPOSE 8000
ENTRYPOINT ["botwork-mcp-echo"]
