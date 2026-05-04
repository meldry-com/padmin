FROM --platform=$BUILDPLATFORM rust:bookworm AS builder

ENV CARGO_HTTP_TIMEOUT=600
ENV CARGO_HTTP_MULTIPLEXING=false
ENV CARGO_NET_RETRY=10
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

ENV RUSTUP_HTTP_TIMEOUT=600
# Install wasm target (with retries for flaky networks)
RUN --network=default \
    for i in 1 2 3 4 5; do rustup target add wasm32-unknown-unknown && break || sleep 10; done
# Pre-install binaryen
RUN apt-get update && apt-get install -y binaryen && rm -rf /var/lib/apt/lists/* || true

RUN --mount=type=cache,id=padmin-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=padmin-cargo-git,target=/usr/local/cargo/git \
    cargo install dioxus-cli@0.7.5 --locked

WORKDIR /app

COPY Cargo.toml Cargo.lock Dioxus.toml ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN --mount=type=cache,id=padmin-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=padmin-cargo-git,target=/usr/local/cargo/git \
    cargo fetch --locked

COPY . .
RUN --network=default \
    --mount=type=cache,id=padmin-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=padmin-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=padmin-target,target=/app/target \
    for i in 1 2 3; do dx build --release && break || echo "Retry $i..." && sleep 10; done && \
    dist_dir="$(find /app/target/dx -type d -path '*/release/web/public' | head -n 1)" && \
    test -n "$dist_dir" && \
    cp -r "$dist_dir" /app/dist

FROM nginxinc/nginx-unprivileged:alpine

# nginxinc/nginx-unprivileged runs as the `nginx` user (UID 101) and listens
# on a high port by default. We let the entrypoint generate a config that
# binds whatever port the operator chose (defaults to 8080 for unprivileged
# nginx — the public reverse proxy in front of padmin should target that).
USER root
COPY --from=builder /app/dist /usr/share/nginx/html
COPY --chmod=755 docker-entrypoint.sh /docker-entrypoint.sh
RUN chown -R nginx:nginx /usr/share/nginx/html /etc/nginx/conf.d
USER nginx

# PALPO_URL / MATRIX_URL: Matrix homeserver internal URL (PALPO_URL takes priority)
# PASION_URL: Pasion auth service internal URL (for Nginx proxy)
# PASION_PUBLIC_URL: Pasion browser-facing URL (for OAuth2 redirect)
# PADMIN_OAUTH_CLIENT_ID: client_id this padmin instance is registered under
# in pasion (`clients[].client_id` in pasion.yaml). Leave unset to use the
# bundle default (`01KMQPADM1N000000000000000`) shipped with examples/.
ENV PALPO_URL=""
ENV MATRIX_URL=""
ENV PASION_URL=""
ENV PASION_PUBLIC_URL=""
ENV PADMIN_OAUTH_CLIENT_ID=""
ENV PADMIN_PORT="8080"

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD wget -q --spider "http://127.0.0.1:${PADMIN_PORT:-8080}/healthz" || exit 1
CMD ["/docker-entrypoint.sh"]
