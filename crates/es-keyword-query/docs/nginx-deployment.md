# Nginx and API deployment

The root `docker-compose.yml` starts an nginx frontend and the existing Rust API. Only nginx publishes a host port:8670 by default. It serves the compiler frontend and proxies `/api/`, `/oauth/`, `/.well-known/` and `/mcp` to the internal API without rewriting their paths or queries. The API continues running as uid10001 with its SQLite database in the `compiler-data` volume. The existing `server/Dockerfile` still builds the standalone server image.

```sh
cp .env.example .env
# Add a private ACCESS_SETUP_TOKEN value to .env before first-account setup.
# Generate it with openssl rand -hex 32; keep .env out of version control.
docker compose up --build -d
```

Open `http://127.0.0.1:8670/first-login` and enter the deployment setup code. Nginx is a nonloopback peer from the API's perspective, so forwarded client addresses never bypass the first-administrator protection. When `ACCESS_SETUP_TOKEN` is absent, the API starts but remote first-account creation stays locked. An explicitly empty token is invalid; remove its line rather than setting it blank. After creating the administrator, remove the token from `.env` and recreate the API with `docker compose up -d --force-recreate api`. Nginx uses Docker's DNS resolver and an upstream shared zone to follow changed API container addresses without restarting the frontend.

Set `ES_PORT` to change the published port. Unless overridden, `ACCESS_PUBLIC_URL` becomes `http://127.0.0.1:<ES_PORT>`. For another hostname or an HTTPS entry point, set it to the exact browser origin, including a nonstandard port. The API currently reads `ACCESS_PUBLIC_URL`; it does not read `ES_SERVER_PUBLIC_URL`. Register provider callbacks as `<ACCESS_PUBLIC_URL>/oauth/callback/github` and `<ACCESS_PUBLIC_URL>/oauth/callback/google`.

Nginx preserves Host including its port, Origin, cookies and bearer Authorization. It overwrites forwarded client/protocol headers and strips the RFC Forwarded header; the API's explicit canonical origin remains authoritative. MCP uses HTTP/1.1 with request/response buffering disabled and300-second proxy timeouts. Missing static assets return404, while browser routes such as `/connections` and `/register` fall back to `index.html`.

Access logs contain the method, URI path, status and size; they omit query strings, cookies and Authorization. Static responses retain frame denial, `nosniff` and same-origin referrer headers; API response headers pass through without duplicates. Main-context startup diagnostics stay on stderr, while server-context error logging goes to `/dev/null`: even critical request failures can include a full OAuth request line. Diagnose request failures from the status in the path-only access log. OAuth authorization codes and state values must never be copied into deployment logs.

```sh
docker compose config --quiet
docker compose exec frontend nginx -t
docker compose ps
docker compose down
```

`down` preserves the data volume. Use `down --volumes` only for a disposable installation whose data can be deleted.

## Verification,2026-11-28

Built the actual Compose services with `docker compose -p es-nginx-verify build`; refreshed the frontend with `docker compose -p es-nginx-verify build frontend` after proxy review. `docker compose config --format json` confirmed a single published frontend port, an internal-only API, the data volume and matching canonical origins for default8670 and custom ports. `docker compose exec -T frontend nginx -t` passed against the running image.

An isolated project on `http://127.0.0.1:19970` passed HTTP and real Chrome checks: static files and JavaScript module MIME, SPA deep links, missing asset404, static security headers without duplicated API headers, uid10001 API, absent-token setup lock, forged-forwarded-header denial, private-token setup with HttpOnly/Lax cookies, default closed registration, pending approval without a session, approval and member login, open member registration, compiler execution and cross-origin403.

Bearer REST and the official rmcp client completed MCP2026-07-28 discovery and a compiler tool call through nginx. Discovery returned the public canonical URLs, and unauthenticated MCP preserved the401 challenge. HTTP/1.1, disabled request/response buffering and300-second read/send timeouts were also checked in the effective nginx configuration.

The test stopped and removed the API, requested an OAuth callback with private query canaries, occupied its old address with an owned helper container, and recreated the API at a different address with the setup token removed. The frontend container stayed unchanged; Docker DNS resolution recovered, and the administrator's persisted session still worked. Normal and upstream-failure requests left no query canary, password, setup code or bearer token in either service's logs.

Final local images were frontend `sha256:2de4bfd754992c57699b333a7ec5f22bb5e26a1e10f9d10ec4587381e78e0f99` and API `sha256:c255ca43d6b6477e3cb125e48633fa4df638cdd8a9c646a2a509f347aa5a30e8`. Session evidence is in `/tmp/es-keyword-query-nginx-compose-smoke.log`, `/tmp/es-keyword-query-nginx-compose-build.log` and `/tmp/es-keyword-query-nginx-frontend-build.log`; the repeatable session harness is `/tmp/es-nginx-compose-smoke.py` and generates its own private test credentials. Its owned containers, helper, network, volume and temporary credential file were removed afterward. This verifies local HTTP proxying; production TLS and live Google/GitHub callbacks still require the intended deployment configuration.
