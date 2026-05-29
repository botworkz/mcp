FROM rust:1.89-bookworm AS builder

WORKDIR /src
COPY echo /src/echo
RUN cd /src/echo && cargo build --release --locked

FROM debian:bookworm-slim

ENV RUST_BACKTRACE=1

COPY --from=builder /src/echo/target/release/botwork-mcp-echo /usr/local/bin/botwork-mcp-echo

EXPOSE 8000
ENTRYPOINT ["botwork-mcp-echo"]
