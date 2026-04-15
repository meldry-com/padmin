#!/bin/sh
set -eu

BACKEND_MATRIX="${PALPO_URL:-$MATRIX_URL}"
RESOLVERS="$(awk '/^nameserver / { print $2 }' /etc/resolv.conf | paste -sd ' ' -)"

if [ -z "$BACKEND_MATRIX" ]; then
    echo "PALPO_URL or MATRIX_URL must be set" >&2
    exit 1
fi

extract_host() {
    url="$1"
    rest="${url#*://}"
    authority="${rest%%/*}"
    printf '%s\n' "${authority%%:*}"
}

can_resolve_url_host() {
    host="$(extract_host "$1")"
    [ -n "$host" ] || return 1
    timeout 2 getent hosts "$host" >/dev/null 2>&1
}

write_proxy_location() {
    location_path="$1"
    target_url="$2"
    auth_header="${3:-}"

    cat >> /etc/nginx/conf.d/default.conf <<EOF
    location ${location_path} {
        proxy_pass ${target_url};
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
EOF

    if [ -n "$auth_header" ]; then
cat >> /etc/nginx/conf.d/default.conf <<EOF
        proxy_set_header Authorization ${auth_header};
EOF
    fi

cat >> /etc/nginx/conf.d/default.conf <<EOF
    }
EOF
}

write_dynamic_proxy_location() {
    location_path="$1"
    variable_name="$2"
    auth_header="${3:-}"

    cat >> /etc/nginx/conf.d/default.conf <<EOF
    location ${location_path} {
        proxy_pass \$${variable_name};
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
EOF

    if [ -n "$auth_header" ]; then
cat >> /etc/nginx/conf.d/default.conf <<EOF
        proxy_set_header Authorization ${auth_header};
EOF
    fi

cat >> /etc/nginx/conf.d/default.conf <<EOF
    }
EOF
}

printf '{"pasion_public_url":"%s"}' "$PASION_PUBLIC_URL" > /usr/share/nginx/html/config.json

cat > /etc/nginx/conf.d/default.conf <<EOF
server {
    listen 80;
    server_name _;
    root /usr/share/nginx/html;
    index index.html;
EOF

if [ -n "$PASION_URL" ]; then
    if can_resolve_url_host "$PASION_URL"; then
        write_proxy_location "/auth/" "$PASION_URL"
        write_proxy_location "/api/v1/auth/" "$PASION_URL"
        write_proxy_location "/api/admin/" "$PASION_URL" "\$http_authorization"
        write_proxy_location "/authorize" "$PASION_URL"
        write_proxy_location "/oauth2/" "$PASION_URL"
        write_proxy_location "/.well-known/" "$PASION_URL"
    else
        [ -n "$RESOLVERS" ] || RESOLVERS="127.0.0.11"
        cat >> /etc/nginx/conf.d/default.conf <<EOF
    resolver ${RESOLVERS} valid=30s ipv6=off;
    set \$pasion_backend ${PASION_URL};
EOF
        write_dynamic_proxy_location "/auth/" "pasion_backend"
        write_dynamic_proxy_location "/api/v1/auth/" "pasion_backend"
        write_dynamic_proxy_location "/api/admin/" "pasion_backend" "\$http_authorization"
        write_dynamic_proxy_location "/authorize" "pasion_backend"
        write_dynamic_proxy_location "/oauth2/" "pasion_backend"
        write_dynamic_proxy_location "/.well-known/" "pasion_backend"
    fi
fi

if can_resolve_url_host "$BACKEND_MATRIX"; then
    write_proxy_location "/_palpo/" "$BACKEND_MATRIX"
    write_proxy_location "/_matrix/" "$BACKEND_MATRIX"
    write_proxy_location "/_synapse/" "$BACKEND_MATRIX"
else
    if ! grep -q "resolver " /etc/nginx/conf.d/default.conf; then
        [ -n "$RESOLVERS" ] || RESOLVERS="127.0.0.11"
        cat >> /etc/nginx/conf.d/default.conf <<EOF
    resolver ${RESOLVERS} valid=30s ipv6=off;
EOF
    fi
    cat >> /etc/nginx/conf.d/default.conf <<EOF
    set \$matrix_backend ${BACKEND_MATRIX};
EOF
    write_dynamic_proxy_location "/_palpo/" "matrix_backend"
    write_dynamic_proxy_location "/_matrix/" "matrix_backend"
    write_dynamic_proxy_location "/_synapse/" "matrix_backend"
fi

cat >> /etc/nginx/conf.d/default.conf <<EOF
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
