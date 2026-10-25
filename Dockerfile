# Build with scripts/package-container.ps1; context is D:/BuildCache/padmin-glory-branch/container.
FROM nginxinc/nginx-unprivileged:alpine

# nginxinc/nginx-unprivileged runs as the `nginx` user (UID 101) and listens
# on a high port by default. We let the entrypoint generate a config that
# binds whatever port the operator chose (defaults to 8080 for unprivileged
# nginx — the public reverse proxy in front of padmin should target that).
USER root
COPY dist /usr/share/nginx/html
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
ENV PADMIN_BASE_PATH=""
ENV PADMIN_PASION_BASE_PATH=""

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD wget -q --spider "http://127.0.0.1:${PADMIN_PORT:-8080}/healthz" || exit 1
CMD ["/docker-entrypoint.sh"]
