# nginx Compose deployment

`docker compose up -d --build` starts nginx and the internal `keyword-vault` API. Open **http://127.0.0.1:8686**. Only nginx publishes `${KV_PORT:-8686}`; the API keeps port `8686` on the Compose network and runs as UID 10001. The existing `keyword-vault-data` volume remains mounted at `/app/data`.

Copy `.env.example` to `.env` for overrides. `KV_PORT` changes both the public port and the default canonical URL. Set `ACCESS_PUBLIC_URL` to the exact browser origin, including a nonstandard port, when using another hostname or external HTTPS. The legacy `KV_PUBLIC_URL` remains supported. Either URL supplies the other when unset; explicitly configured values must have matching origins. Provider callbacks stay `/oauth/callback/github` and `/oauth/callback/google` on that same origin. External HTTPS termination is configured outside this Compose file; nginx does not trust caller-supplied forwarding headers.

nginx serves the built React assets, returns the SPA for client routes such as `/register`, and returns 404 for missing `/assets/` files. It proxies `/api`, `/oauth`, `/.well-known` and `/mcp` without stripping prefixes or rewriting queries. Host (including port), Origin, Authorization and Cookie are preserved. Forwarding headers are replaced with nginx's socket peer and scheme. HTTP/1.1, disabled request/response buffering and proxy timeouts of 300 seconds support MCP streaming. The global body limit is 16 MiB; application-specific limits still apply. Access logs record `$uri`, never OAuth query strings.

Check the proxy and application separately:

```sh
docker compose config
docker compose up -d --build
curl --fail http://127.0.0.1:8686/healthz
curl --fail http://127.0.0.1:8686/api/health
curl --fail http://127.0.0.1:8686/api/auth/registration-policy
docker compose ps
docker compose logs --tail=50 nginx
```

`/healthz` checks nginx itself; `/api/health` reaches the real backend. Preserve the named data volume during upgrades: `docker compose down` stops services, while `docker compose down -v` deletes the database and is only appropriate for disposable test deployments.

The Dockerfile has three runnable targets: `runtime` (API only), `frontend` (nginx + built React), and final/default `standalone` (the original nonroot API + static files). Existing direct builds remain supported:

```sh
docker build -t keyword-vault:standalone .
```

Compose builds the two smaller deployment targets explicitly. The backend remains reachable to other services on the Compose network, rather than through a published host port. Application authorization and registration policy remain enforced by the backend.

Proxy semantics follow the [nginx proxy module](https://nginx.org/en/docs/http/ngx_http_proxy_module.html#proxy_pass) and the path-only access format uses the [nginx log module](https://nginx.org/en/docs/http/ngx_http_log_module.html#log_format).


Static success and error responses include DENY, nosniff and same-origin security headers. Each static location includes the full header set so nginx inheritance does not drop a header; proxied responses retain the backend headers without duplicate additions. Missing images, favicons and other static file requests return 404 instead of SPA HTML. Docker DNS re-resolves the backend every 5 seconds, allowing a replaced backend container to recover without restarting nginx.

Server-scoped nginx error logging is disabled because request-bearing errors can include OAuth code/state query strings even at critical severity. Main-process startup/configuration errors still use the nginx image's stderr logger. The safe access log retains method, path, status, upstream status and connection/response times to diagnose backend failures without logging queries or credentials.

## Actual Compose verification,2026-11-28

`docker compose build` built the final API and nginx targets. API image: `sha256:e47ef99a23cd5278711044ae18481f5f64665cdb51febd93ad316549ae54e196`; nginx image: `sha256:f6817aa016b49cad32168bd6fbf36de3d7b6fab42bfe6a5f6e1475c4dd963bd4`. Build log: `/tmp/keyword-vault-nginx-compose-build.log`.

`/Users/natlee/miniconda3/bin/python3.12 /tmp/verify-nginx-compose.py --skip-build keyword-vault` passed against a fresh, isolated Compose project at `http://127.0.0.1:28686` in5.7seconds. It checked nginx/backend health, static security headers and missing-file404, closed/approval/rejected/open registration, pending denial, approval, stale review409 and member policy denial. Bearer REST plus MCP2026-07-28 discover/list/call ran the real `search_keywords` operation; revoking the key immediately produced401.

Installed Chrome158.0.8127.37 exercised the built React app through nginx with a64-character username,256-character password and100-character display name, then edited the profile, signed out and signed back in. The API ran nonroot with no published host port. Recreating only the backend preserved the administrator's database session while the nginx container stayed unchanged. No test password or bearer key appeared in nginx logs.

The harness writes `/tmp/keyword-vault-nginx-compose.result.json`, `.summary.log`, `.runtime.log` and `.cleanup.log`. It removed its own containers, network, fresh data volume and temporary override afterward. These are local HTTP/protocol/browser checks; they do not claim production TLS or live provider callbacks.


## 統一編譯器版本的初始化與詞典

此版本的 nginx 連線屬於遠端 socket。全新資料庫須透過 `ACCESS_SETUP_TOKEN` 保護首次設定；部署前用 `openssl rand -hex 32` 產生32–512字元權杖並設於服務環境，首次表單輸入同值。完成後移除環境變數並重新建立 API 容器；已有帳號不再接受初始化。不要設定空字串。也可以先用 `docker compose run --rm -T keyword-vault keyword-vault bootstrap <帳號>` 從 stdin安全提供密碼，建立管理員後再啟動對外入口。不要把密碼當命令列參數。

Compose保留 `ACCESS_SETUP_TOKEN` 的未設定狀態，不使用固定預設值。CI／smoke測試會臨時產生獨立權杖；執行 `docs/smoke-test.sh` 時傳入同一環境變數即可。舊 Vault資料庫可原地升級，編譯器來源資料庫只能經 [離線匯入](compiler-migration.md) 處理。

內建繁體詞典直接包進 API binary，runtime不需連線下載。若需自訂詞典，在 Compose override 中為 API 加上唯讀bind mount，例如 `/srv/keyword-vault/user-dict.txt:/app/config/user-dict.txt:ro`，並設 `KV_SEGMENTATION_DICT=/app/config/user-dict.txt`。容器使用uid10001，檔案需對該uid可讀。格式錯誤會阻止啟動；變更後須重新啟動並核對預覽回傳的dict SHA-256。
