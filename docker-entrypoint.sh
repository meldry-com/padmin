#!/bin/sh
set -eu

BACKEND_MATRIX="${PALPO_URL:-$MATRIX_URL}"
PADMIN_PORT="${PADMIN_PORT:-80}"
PADMIN_BASE_PATH="${PADMIN_BASE_PATH:-}"
PADMIN_PASION_BASE_PATH="${PADMIN_PASION_BASE_PATH:-}"
# These paths are interpolated into nginx, JavaScript and sed expressions.
# Accept nested URL segments without permitting directives or regex syntax.
normalize_mount() {
    mount="${1%/}"
    if [ -n "$mount" ] && ! printf '%s' "$mount" | grep -Eq '^(/[A-Za-z0-9_-]+)+$'; then
        echo 'Mount paths must be empty or slash-separated letters, digits, underscores and hyphens' >&2
        exit 1
    fi
    printf '%s' "$mount"
}
PADMIN_BASE_PATH="$(normalize_mount "$PADMIN_BASE_PATH")"
PADMIN_PASION_BASE_PATH="$(normalize_mount "$PADMIN_PASION_BASE_PATH")"
RESOLVERS="$(awk '/^nameserver / { print $2 }' /etc/resolv.conf | paste -sd ' ' -)"
LOOKUP_UNAVAILABLE=0

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
    [ "$LOOKUP_UNAVAILABLE" -eq 0 ] || return 1

    if timeout 0.5 getent hosts "$host" >/dev/null 2>&1; then
        return 0
    fi

    status=$?
    LOOKUP_UNAVAILABLE=1
    return 1
}

write_auth_asset_filter() {
    if [ "$location_path" = /auth/ ] || { [ -n "$PADMIN_PASION_BASE_PATH" ] && [ "$location_path" = "${PADMIN_PASION_BASE_PATH}/" ]; }; then
        cat >> /etc/nginx/conf.d/default.conf <<EOF
        # Some Pasion distributions contain absolute JS-to-WASM asset paths.
        # Keep those requests within the explicitly configured auth mount.
        proxy_set_header Accept-Encoding "";
        sub_filter_types application/javascript text/javascript;
        sub_filter_once off;
        sub_filter '/./assets/' '${location_path}assets/';
        sub_filter '"/assets/' '"${location_path}assets/';
        # Keep the inherited CSP while making the upstream's public inline
        # configuration inert. The external bridge uses JSON.parse, never eval.
        sub_filter '<script>' '<script type="application/json" id="padmin-pasion-config">';
        sub_filter '</script>' '</script><script src="${PADMIN_BASE_PATH}/pasion-config.js"></script>';
EOF
    fi
}

write_proxy_location() {
    location_path="$1"
    target_url="$2"
    auth_header="${3:-}"
    # The browser auth mount is a reverse-proxy prefix, not an upstream route.
    if [ "$location_path" = /auth/ ] || { [ -n "$PADMIN_PASION_BASE_PATH" ] && [ "$location_path" = "${PADMIN_PASION_BASE_PATH}/" ]; }; then
        target_url="${target_url%/}/"
    fi

    cat >> /etc/nginx/conf.d/default.conf <<EOF
    location ^~ ${location_path} {
        proxy_pass ${target_url};
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
EOF
    write_auth_asset_filter

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
    location ^~ ${location_path} {
EOF
    if [ "$location_path" = /auth/ ] || { [ -n "$PADMIN_PASION_BASE_PATH" ] && [ "$location_path" = "${PADMIN_PASION_BASE_PATH}/" ]; }; then
        cat >> /etc/nginx/conf.d/default.conf <<EOF
        rewrite ^${location_path}(.*)\$ /\$1 break;
EOF
    fi
    cat >> /etc/nginx/conf.d/default.conf <<EOF
        proxy_pass \$${variable_name};
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
EOF
    write_auth_asset_filter

    if [ -n "$auth_header" ]; then
cat >> /etc/nginx/conf.d/default.conf <<EOF
        proxy_set_header Authorization ${auth_header};
EOF
    fi

cat >> /etc/nginx/conf.d/default.conf <<EOF
    }
EOF
}

PADMIN_OAUTH_CLIENT_ID="${PADMIN_OAUTH_CLIENT_ID:-}"
# Keep pristine files so container restarts never double-prefix asset URLs.
for file in /usr/share/nginx/html/index.html; do
    [ -f "$file" ] || continue
    [ -f "$file.original" ] || cp "$file" "$file.original"
    cp "$file.original" "$file"
done
printf "window.PADMIN_BASE_PATH='%s';window.PADMIN_PASION_BASE_PATH='%s';\n" "$PADMIN_BASE_PATH" "$PADMIN_PASION_BASE_PATH" > /usr/share/nginx/html/runtime-config.js
sed -i "s@<base href=\"/\">@<base href=\"${PADMIN_BASE_PATH}/\">@" /usr/share/nginx/html/index.html
printf '{"pasion_public_url":"%s","oauth_client_id":"%s"}' \
    "$PASION_PUBLIC_URL" "$PADMIN_OAUTH_CLIENT_ID" \
    > /usr/share/nginx/html/config.json

cat > /etc/nginx/conf.d/default.conf <<EOF
server {
    listen ${PADMIN_PORT};
    server_name _;
    absolute_redirect off;
    root /usr/share/nginx/html;
    index index.html;

    # Baseline security response headers. \`always\` makes nginx emit them on
    # error pages too, not just 2xx/3xx. CSP allows \`wasm-unsafe-eval\`
    # because Glory runs as WebAssembly.
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    add_header Permissions-Policy "accelerometer=(), camera=(), geolocation=(), microphone=(), payment=(), usb=()" always;
    add_header Cross-Origin-Opener-Policy "same-origin" always;
    add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; img-src 'self' data: blob: https:; font-src 'self' data: https://fonts.gstatic.com; connect-src 'self' ${PASION_PUBLIC_URL}; frame-ancestors 'none'; base-uri 'self'; form-action 'self'" always;

    location = /healthz {
        access_log off;
        default_type text/plain;
        return 200 'ok';
    }
    location = /runtime-config.js {
        expires -1;
    }
EOF

if [ -n "$PASION_URL" ]; then
    if can_resolve_url_host "$PASION_URL"; then
        if [ -n "$PADMIN_PASION_BASE_PATH" ] && [ "$PADMIN_PASION_BASE_PATH" != /auth ]; then
            write_proxy_location "${PADMIN_PASION_BASE_PATH}/" "$PASION_URL"
        fi
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
        if [ -n "$PADMIN_PASION_BASE_PATH" ] && [ "$PADMIN_PASION_BASE_PATH" != /auth ]; then
            write_dynamic_proxy_location "${PADMIN_PASION_BASE_PATH}/" "pasion_backend"
        fi
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

if [ -n "$PADMIN_BASE_PATH" ]; then
cat >> /etc/nginx/conf.d/default.conf <<EOF
    location = ${PADMIN_BASE_PATH} {
        return 308 ${PADMIN_BASE_PATH}/\$is_args\$args;
    }

    location ^~ ${PADMIN_BASE_PATH}/pkg/ {
        alias /usr/share/nginx/html/pkg/;
        expires -1;
    }

    location ~ ^${PADMIN_BASE_PATH}/(.+\\.(wasm|js|css|png|jpg|ico|svg))\$ {
        rewrite ^${PADMIN_BASE_PATH}/(.*)\$ /\$1 break;
        try_files \$uri =404;
        expires -1;
    }

    location ${PADMIN_BASE_PATH}/ {
        rewrite ^${PADMIN_BASE_PATH}/(.*)\$ /\$1 break;
        try_files \$uri \$uri/ /index.html;
        expires -1;
    }
EOF
fi

cat >> /etc/nginx/conf.d/default.conf <<EOF
    location ~* \.(wasm|js|css|png|jpg|ico|svg)$ {
        try_files \$uri =404;
        expires -1;
    }

    location / {
        try_files \$uri \$uri/ /index.html;
    }
}
EOF

nginx -t
exec nginx -g "daemon off;"
