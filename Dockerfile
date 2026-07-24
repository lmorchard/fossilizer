# syntax=docker/dockerfile:1

# ---- Builder: compile the release binary ----
FROM rust:1-bookworm AS builder
WORKDIR /usr/src/fossilizer
# buildpack-deps (rust image base) already provides gcc, make, and pkg-config
# needed for the bundled-sqlite build. (TLS is rustls now, so no OpenSSL/libssl.)
COPY . .
RUN cargo build --release --locked

# ---- Pagefind: fetch the pinned search-index binary ----
FROM debian:bookworm-slim AS pagefind
ARG PAGEFIND_VERSION=v1.5.2
ARG TARGETARCH
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN set -eux; \
    case "$TARGETARCH" in \
      amd64) target=x86_64-unknown-linux-musl ;; \
      arm64) target=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported TARGETARCH: ${TARGETARCH:-unset}" >&2; exit 1 ;; \
    esac; \
    url="https://github.com/CloudCannon/pagefind/releases/download/${PAGEFIND_VERSION}/pagefind-${PAGEFIND_VERSION}-${target}.tar.gz"; \
    curl -fsSL "$url" -o /tmp/pagefind.tar.gz; \
    tar -xzf /tmp/pagefind.tar.gz -C /usr/local/bin; \
    chmod +x /usr/local/bin/pagefind; \
    /usr/local/bin/pagefind --version

# ---- Runtime: assemble the slim final image ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/fossilizer/target/release/fossilizer /usr/local/bin/fossilizer
COPY --from=pagefind /usr/local/bin/pagefind /usr/local/bin/pagefind
COPY docker/backup-loop.sh /usr/local/bin/backup-loop.sh
RUN chmod +x /usr/local/bin/backup-loop.sh
ENV APP_DATA_PATH=/data \
    APP_BUILD_PATH=/build
ENTRYPOINT ["fossilizer"]
