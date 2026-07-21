ARG BINARY_SOURCE=source


FROM rust:1.89-bookworm AS builder
WORKDIR /workspace/echo
COPY echo /workspace/echo
COPY VERSION /workspace
# Surface the git sha to the cargo build so botwork-version's
# option_env!("BOTWORK_GIT_SHA") picks it up at compile time.
# Empty in local-dev builds (no --build-arg), populated by CI.
ARG BOTWORK_GIT_SHA=""
ENV BOTWORK_GIT_SHA=${BOTWORK_GIT_SHA}
RUN cargo build --release --locked


FROM scratch AS source
COPY --from=builder /src/echo/target/release/botwork-mcp-echo /botwork-mcp-echo


FROM scratch AS prebuilt
COPY .ci-prebuilt/echo/botwork-mcp-echo /botwork-mcp-echo


FROM ${BINARY_SOURCE} AS binary-source


FROM gcr.io/distroless/base-nossl-debian12:nonroot
ARG GIT_SHA=""
ARG BOTWORK_VERSION=""
LABEL org.opencontainers.image.revision="${GIT_SHA}"
LABEL org.opencontainers.image.source="https://github.com/botworkz/mcp"
LABEL org.opencontainers.image.version="${BOTWORK_VERSION}"
ARG BINARY_SOURCE=source
ENV RUST_BACKTRACE=1
EXPOSE 8000
ENTRYPOINT ["/usr/local/bin/botwork-mcp-echo"]
COPY --from=builder /lib/x86_64-linux-gnu/libgcc_s.so.1 \
    /lib/x86_64-linux-gnu/libgcc_s.so.1
COPY --from=binary-source /botwork-mcp-echo /usr/local/bin/botwork-mcp-echo
