FROM rust:bookworm AS builder

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
    cargo install dioxus-cli@0.7.4 --locked

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
    cp -r /app/target/dx/palpo-admin/release/web/public /app/dist

FROM nginx:alpine

COPY --from=builder /app/dist /usr/share/nginx/html

# PALPO_URL / MATRIX_URL: Matrix homeserver internal URL (PALPO_URL takes priority)
# PASION_URL: Pasion auth service internal URL (for Nginx proxy)
# PASION_PUBLIC_URL: Pasion browser-facing URL (for OAuth2 redirect)
ENV PALPO_URL=""
ENV MATRIX_URL=""
ENV PASION_URL=""
ENV PASION_PUBLIC_URL=""

# Entrypoint: generate nginx config + browser config from env vars
# Uses nginx variables for proxy_pass so upstream DNS is resolved at request
# time (not startup), preventing crashes when backends start slowly.
RUN printf '#!/bin/sh\nset -e\nBACKEND_MATRIX="${PALPO_URL:-$MATRIX_URL}"\n\
printf '"'"'{"pasion_public_url":"%%s"}'"'"' "$PASION_PUBLIC_URL" > /usr/share/nginx/html/config.json\n\
cat > /etc/nginx/conf.d/default.conf <<NGINX_EOF\n\
server {\n\
    listen 80;\n\
    server_name _;\n\
    root /usr/share/nginx/html;\n\
    index index.html;\n\
    resolver 127.0.0.11 valid=30s ipv6=off;\n\
    set \\$pasion_backend ${PASION_URL};\n\
    set \\$matrix_backend ${BACKEND_MATRIX};\n\
    location /auth/ {\n\
        proxy_pass \\$pasion_backend;\n\
        proxy_set_header Host \\$host;\n\
        proxy_set_header X-Real-IP \\$remote_addr;\n\
    }\n\
    location /api/v1/auth/ {\n\
        proxy_pass \\$pasion_backend;\n\
        proxy_set_header Host \\$host;\n\
        proxy_set_header X-Real-IP \\$remote_addr;\n\
    }\n\
    location /api/admin/ {\n\
        proxy_pass \\$pasion_backend;\n\
        proxy_set_header Host \\$host;\n\
        proxy_set_header X-Real-IP \\$remote_addr;\n\
        proxy_set_header Authorization \\$http_authorization;\n\
    }\n\
    location /authorize {\n\
        proxy_pass \\$pasion_backend;\n\
        proxy_set_header Host \\$host;\n\
        proxy_set_header X-Real-IP \\$remote_addr;\n\
    }\n\
    location /oauth2/ {\n\
        proxy_pass \\$pasion_backend;\n\
        proxy_set_header Host \\$host;\n\
        proxy_set_header X-Real-IP \\$remote_addr;\n\
    }\n\
    location /.well-known/ {\n\
        proxy_pass \\$pasion_backend;\n\
        proxy_set_header Host \\$host;\n\
        proxy_set_header X-Real-IP \\$remote_addr;\n\
    }\n\
    location /_palpo/ {\n\
        proxy_pass \\$matrix_backend;\n\
        proxy_set_header Host \\$host;\n\
        proxy_set_header X-Real-IP \\$remote_addr;\n\
    }\n\
    location /_matrix/ {\n\
        proxy_pass \\$matrix_backend;\n\
        proxy_set_header Host \\$host;\n\
        proxy_set_header X-Real-IP \\$remote_addr;\n\
    }\n\
    location /_synapse/ {\n\
        proxy_pass \\$matrix_backend;\n\
        proxy_set_header Host \\$host;\n\
        proxy_set_header X-Real-IP \\$remote_addr;\n\
    }\n\
    location ~* \\.(wasm|js|css|png|jpg|ico|svg)\\$ {\n\
        expires 1y;\n\
        add_header Cache-Control "public, immutable";\n\
    }\n\
    location / {\n\
        try_files \\$uri \\$uri/ /index.html;\n\
    }\n\
}\n\
NGINX_EOF\n\
exec nginx -g "daemon off;"\n' > /docker-entrypoint.sh && chmod +x /docker-entrypoint.sh

EXPOSE 80
CMD ["/docker-entrypoint.sh"]
