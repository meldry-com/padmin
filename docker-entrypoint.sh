#!/bin/sh
set -eu

BACKEND_MATRIX="${PALPO_URL:-$MATRIX_URL}"

if [ -z "$BACKEND_MATRIX" ]; then
    echo "PALPO_URL or MATRIX_URL must be set" >&2
    exit 1
fi

printf '{"pasion_public_url":"%s"}' "$PASION_PUBLIC_URL" > /usr/share/nginx/html/config.json

cat > /etc/nginx/conf.d/default.conf <<EOF
server {
    listen 80;
    server_name _;
    root /usr/share/nginx/html;
    index index.html;
    resolver 127.0.0.11 valid=30s ipv6=off;
    set \$matrix_backend ${BACKEND_MATRIX};
EOF

if [ -n "$PASION_URL" ]; then
cat >> /etc/nginx/conf.d/default.conf <<EOF
    set \$pasion_backend ${PASION_URL};

    location /auth/ {
        proxy_pass \$pasion_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }

    location /api/v1/auth/ {
        proxy_pass \$pasion_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }

    location /api/admin/ {
        proxy_pass \$pasion_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header Authorization \$http_authorization;
    }

    location /authorize {
        proxy_pass \$pasion_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }

    location /oauth2/ {
        proxy_pass \$pasion_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }

    location /.well-known/ {
        proxy_pass \$pasion_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }
EOF
fi

cat >> /etc/nginx/conf.d/default.conf <<EOF
    location /_palpo/ {
        proxy_pass \$matrix_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }

    location /_matrix/ {
        proxy_pass \$matrix_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }

    location /_synapse/ {
        proxy_pass \$matrix_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }

    location ~* \.(wasm|js|css|png|jpg|ico|svg)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    location / {
        try_files \$uri \$uri/ /index.html;
    }
}
EOF

nginx -t
exec nginx -g "daemon off;"
