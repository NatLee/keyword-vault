import { useEffect, useRef, useState } from "react";
import { Copy, KeyRound, Link2, RefreshCw, ShieldCheck, X } from "lucide-react";
import { useUser } from "../context";
import { ErrorBanner } from "../components/ui";
import {
  accessRequest,
  SCOPE_LABEL,
  utc,
  providerError,
  type AccessEvent,
  type Client,
  type Credential,
  type Overview,
  type Provider,
  type Usage,
} from "../lib/access";
import "../access.css";
const scopes = Object.keys(SCOPE_LABEL);
export function Access() {
  const user = useUser();
  const [data, setData] = useState<Overview | null>(null);
  const [providers, setProviders] = useState<Provider[]>([]);
  const [usage, setUsage] = useState<Usage | null>(null);
  const [events, setEvents] = useState<AccessEvent[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [clients, setClients] = useState<Client[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [secret, setSecret] = useState<string | null>(null);
  const [notice, setNotice] = useState("");
  const [name, setName] = useState("");
  const [days, setDays] = useState(30);
  const [selected, setSelected] = useState(scopes.filter((scope) => scope !== "queries:read"));
  const [audiences, setAudiences] = useState(["api", "mcp"]);
  const [period, setPeriod] = useState(7);
  const [transport, setTransport] = useState("");
  const [outcome, setOutcome] = useState("");
  const [owner, setOwner] = useState("");
  const [clientName, setClientName] = useState("");
  const [redirect, setRedirect] = useState("");
  const createButton = useRef<HTMLButtonElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const closeSecret = () => {
    setSecret(null);
    createButton.current?.focus();
  };
  useEffect(() => {
    if (secret) closeButton.current?.focus();
  }, [secret]);
  const version = useRef(0);
  const appliedQuery = useRef("");
  const pagination = useRef<symbol | null>(null);
  const [loadingPage, setLoadingPage] = useState(false);
  const [ownerDraft, setOwnerDraft] = useState("");
  const query = () =>
    new URLSearchParams({
      days: String(period),
      ...(transport ? { transport } : {}),
      ...(user.role === "admin" && owner ? { user_id: owner } : {}),
      ...(outcome ? { outcome } : {}),
    }).toString();
  async function reload() {
    const request = ++version.current;
    const requestedQuery = query();
    pagination.current = null;
    setLoadingPage(false);
    setCursor(null);
    setError("");
    try {
      const [o, p, u, e] = await Promise.all([
        accessRequest<Overview>("/overview"),
        accessRequest<{ items: Provider[] }>("/providers"),
        accessRequest<Usage>(`/usage?${requestedQuery}`),
        accessRequest<{ items: AccessEvent[]; next_cursor: string | null }>(
          `/events?${requestedQuery}`,
        ),
      ]);
      if (request !== version.current) return;
      setData(o);
      setProviders(p.items);
      setUsage(u);
      appliedQuery.current = requestedQuery;
      setEvents(e.items);
      setCursor(e.next_cursor);
      if (user.role === "admin") {
        const registered = await accessRequest<{ items: Client[] }>("/clients");
        if (request === version.current) setClients(registered.items);
      }
    } catch (e) {
      if (request === version.current) setError((e as Error).message);
    }
  }
  useEffect(() => {
    void reload();
  }, [period, transport, outcome, owner]);
  useEffect(
    () => () => {
      version.current++;
    },
    [],
  );
  async function loadEarlier() {
    if (!cursor || pagination.current) return;
    const request = version.current;
    const requestedQuery = appliedQuery.current;
    const requestedCursor = cursor;
    const page = Symbol("pagination");
    pagination.current = page;
    setLoadingPage(true);
    try {
      const next = await accessRequest<{
        items: AccessEvent[];
        next_cursor: string | null;
      }>(
        `/events?${requestedQuery}&cursor=${encodeURIComponent(requestedCursor)}`,
      );
      if (request !== version.current || pagination.current !== page) return;
      setEvents((current) => {
        const seen = new Set(current.map((event) => event.id));
        const additions = next.items.filter((event) => {
          if (seen.has(event.id)) return false;
          seen.add(event.id);
          return true;
        });
        return [...current, ...additions];
      });
      setCursor(next.next_cursor);
    } catch (e) {
      if (request === version.current && pagination.current === page)
        setError((e as Error).message);
    } finally {
      if (pagination.current === page) {
        pagination.current = null;
        setLoadingPage(false);
      }
    }
  }
  async function action(work: () => Promise<void>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await work();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const toggle = (values: string[], value: string) =>
    values.includes(value)
      ? values.filter((v) => v !== value)
      : [...values, value];
  async function create() {
    await action(async () => {
      const result = await accessRequest<{
        credential: Credential;
        token: string;
      }>("/credentials", "POST", {
        name: name.trim(),
        scopes: selected,
        audiences,
        expires_in_days: days,
      });
      setSecret(result.token);
      setName("");
      setData((current) =>
        current
          ? {
              ...current,
              credentials: [result.credential, ...current.credentials],
            }
          : current,
      );
    });
  }
  async function revoke(
    kind: "credentials" | "grants" | "identities",
    id: string,
  ) {
    await action(async () => {
      await accessRequest(`/${kind}/${encodeURIComponent(id)}`, "DELETE");
      setNotice("已撤銷；後續請求會重新檢查權限。");
      await reload();
    });
  }
  const max = Math.max(1, ...(usage?.daily.map((d) => d.attempts) ?? []));
  return (
    <div className="access-desk">
      <header className="access-heading">
        <div>
          <p className="eyebrow">ACCESSION REGISTER / 存取登錄簿</p>
          <h1>連線與存取</h1>
          <p>管理自己的閱覽證、應用授權與實際存取紀錄。</p>
        </div>
        <button className="btn btn-secondary" onClick={() => void reload()}>
          <RefreshCw className="size-4" />
          重新整理
        </button>
      </header>
      {providerError() && <ErrorBanner message={providerError()!} />}{" "}
      {error && <ErrorBanner message={error} />}{" "}
      {notice && (
        <p role="status" className="access-notice">
          {notice}
        </p>
      )}
      <section className="access-section">
        <header>
          <span>01 / IDENTITY</span>
          <h2>
            <Link2 className="size-5" />
            帳號綁定
          </h2>
        </header>
        <p>
          目前閱覽者：{user.display_name}
          。第三方帳號只綁定此帳號，不會依電子郵件合併。
        </p>
        <div className="access-providers">
          {providers.map((p) => {
            const identity = data?.identities.find(
              (i) => i.provider === p.provider,
            );
            return (
              <article key={p.provider}>
                <h3>{p.name}</h3>
                {identity ? (
                  <>
                    <p>{identity.subject_mask}</p>
                    <p>綁定於 {utc(identity.linked_at)}</p>
                    <button
                      className="btn btn-secondary"
                      disabled={busy}
                      onClick={() => void revoke("identities", identity.id)}
                    >
                      解除 {p.name}
                    </button>
                  </>
                ) : (
                  <>
                    <p>
                      {p.enabled
                        ? "尚未綁定"
                        : `${p.name} 尚未設定；請管理員配置供應商。`}
                    </p>
                    <button
                      className="btn btn-secondary"
                      disabled={!p.enabled || busy}
                      onClick={() =>
                        void action(async () => {
                          const r = await accessRequest<{
                            authorization_url: string;
                          }>(
                            `/identities/${encodeURIComponent(p.provider)}/link`,
                            "POST",
                            {},
                          );
                          window.location.assign(r.authorization_url);
                        })
                      }
                    >
                      綁定 {p.name}
                    </button>
                  </>
                )}
              </article>
            );
          })}
        </div>
      </section>
      <section className="access-section access-credentials">
        <header>
          <span>02 / PERSONAL KEYS</span>
          <h2>
            <KeyRound className="size-5" />
            API 金鑰
          </h2>
        </header>
        <div className="access-columns">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void create();
            }}
          >
            <label className="label" htmlFor="key-name">
              金鑰名稱
            </label>
            <input
              id="key-name"
              className="input"
              required
              maxLength={80}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <label className="label" htmlFor="key-days">
              有效天數
            </label>
            <input
              id="key-days"
              className="input"
              type="number"
              min={1}
              max={90}
              value={days}
              onChange={(e) => setDays(Number(e.target.value))}
            />
            <fieldset>
              <legend>允許的權限</legend>
              {scopes.map((scope) => (
                <label key={scope}>
                  <input
                    type="checkbox"
                    checked={selected.includes(scope)}
                    onChange={() => setSelected(toggle(selected, scope))}
                  />
                  {SCOPE_LABEL[scope]} <code>{scope}</code>
                </label>
              ))}
            </fieldset>
            <fieldset>
              <legend>允許的通道</legend>
              {["api", "mcp"].map((v) => (
                <label key={v}>
                  <input
                    type="checkbox"
                    checked={audiences.includes(v)}
                    onChange={() => setAudiences(toggle(audiences, v))}
                  />
                  {v.toUpperCase()}
                </label>
              ))}
            </fieldset>
            <button
              ref={createButton}
              className="btn btn-primary"
              disabled={busy || !selected.length || !audiences.length}
            >
              建立 API 金鑰
            </button>
            <p className="hint">
              最多 90
              天。撤銷、停用帳號或移除群組後，既有金鑰立即失去對應存取能力。
            </p>
          </form>
          <div>
            <p>原值只在建立後顯示一次；清單僅保留前綴與期限。</p>
            {data?.credentials.length === 0 && (
              <p className="access-empty">尚未建立 API 金鑰。</p>
            )}
            {data?.credentials.map((c) => (
              <article className="access-record" key={c.id}>
                <h3>{c.name}</h3>
                <code>{c.prefix}…</code>
                <p>
                  {c.scopes.join(" · ")} /{" "}
                  {c.audiences.join(" · ").toUpperCase()}
                </p>
                <p>到期（UTC）：{utc(c.expires_at)}</p>
                <p>最後使用（UTC）：{utc(c.last_used_at)}</p>
                {c.revoked_at ? (
                  <p>已撤銷</p>
                ) : (
                  <button
                    className="btn btn-secondary"
                    disabled={busy}
                    onClick={() => void revoke("credentials", c.id)}
                  >
                    撤銷 {c.name}
                  </button>
                )}
              </article>
            ))}
          </div>
        </div>
      </section>
      {secret && (
        <section
          className="access-secret"
          onKeyDown={(e) => {
            if (e.key === "Escape") closeSecret();
          }}
          role="dialog"
          aria-modal="false"
          aria-labelledby="key-secret-title"
        >
          <h2 id="key-secret-title">請保存這次建立的金鑰</h2>
          <p>關閉後無法再次讀取原值。</p>
          <code>{secret}</code>
          <div>
            <button
              className="btn btn-secondary"
              onClick={() =>
                void navigator.clipboard.writeText(secret).then(
                  () => setNotice("已複製金鑰。"),
                  () => setNotice("請選取原值手動複製。"),
                )
              }
            >
              <Copy className="size-4" />
              複製金鑰
            </button>
            <button
              className="btn btn-primary"
              ref={closeButton}
              aria-label="關閉金鑰原值"
              onClick={closeSecret}
            >
              <X className="size-4" />
              關閉原值
            </button>
          </div>
        </section>
      )}
      <section className="access-section">
        <header>
          <span>03 / APPLICATION GRANTS</span>
          <h2>
            <ShieldCheck className="size-5" />
            應用授權
          </h2>
        </header>
        {data?.grants.length === 0 && (
          <p className="access-empty">尚未授權任何應用程式。</p>
        )}
        {data?.grants.map((g) => (
          <article className="access-record" key={g.id}>
            <h3>{g.client_name}</h3>
            <p>{g.scopes.join(" · ")}</p>
            <code>{g.resource}</code>
            <p>到期（UTC）：{utc(g.expires_at)}</p>
            {g.revoked_at ? (
              <p>已撤銷</p>
            ) : (
              <button
                className="btn btn-secondary"
                disabled={busy}
                onClick={() => void revoke("grants", g.id)}
              >
                撤銷應用 {g.client_name}
              </button>
            )}
          </article>
        ))}
      </section>
      <section className="access-section access-ledger">
        <header>
          <span>04 / USAGE AND AUDIT</span>
          <h2>用量與存取紀錄</h2>
        </header>
        <div className="access-filters">
          <label>
            日期範圍
            <select
              className="input"
              value={period}
              onChange={(e) => setPeriod(Number(e.target.value))}
            >
              {[7, 30, 90].map((n) => (
                <option key={n} value={n}>
                  最近 {n} 天
                </option>
              ))}
            </select>
          </label>
          <label>
            通道
            <select
              className="input"
              value={transport}
              onChange={(e) => setTransport(e.target.value)}
            >
              <option value="">API 與 MCP</option>
              <option value="api">API</option>
              <option value="mcp">MCP</option>
            </select>
          </label>
          <label>
            結果
            <select
              className="input"
              value={outcome}
              onChange={(e) => setOutcome(e.target.value)}
            >
              <option value="">全部</option>
              {["success", "failure", "pending", "interrupted"].map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </label>
          {user.role === "admin" && (
            <label>
              使用者編號（空白為本人／all 為全體）
              <input
                className="input"
                value={ownerDraft}
                onChange={(e) => setOwnerDraft(e.target.value)}
                maxLength={24}
              />
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => setOwner(ownerDraft.trim())}
              >
                套用使用者
              </button>
            </label>
          )}
        </div>
        {usage && (
          <>
            <p>UTC · 最近 {usage.days} 天</p>
            <p className="hint">
              {utc(usage.from)} — {utc(usage.to)}。僅計領域操作；MCP
              協定探索不計次。
            </p>
            <dl className="access-totals">
              {(
                [
                  ["attempts", "嘗試"],
                  ["successes", "成功"],
                  ["failures", "失敗"],
                  ["pending", "待完成"],
                ] as const
              ).map(([key, label]) => (
                <div key={key}>
                  <dt>{label}</dt>
                  <dd>{usage.totals[key]}</dd>
                </div>
              ))}
            </dl>
            <div
              className="access-bars"
              role="img"
              aria-label={`最近 ${usage.days} 天每日實際操作次數`}
              style={{
                gridTemplateColumns: `repeat(${usage.daily.length},minmax(0,1fr))`,
              }}
            >
              {usage.daily.map((d) => (
                <div
                  key={d.date}
                  title={`${d.date} · 嘗試 ${d.attempts} / 成功 ${d.successes} / 失敗 ${d.failures} / 待完成 ${d.pending}`}
                >
                  <span style={{ height: `${(d.attempts / max) * 100}%` }} />
                  <small>{d.date.slice(5)}</small>
                </div>
              ))}
            </div>
            {usage.totals.attempts === 0 && <p>此範圍尚無操作紀錄。</p>}
          </>
        )}
        <div className="access-table">
          <table>
            <thead>
              <tr>
                <th>時間（UTC）</th>
                <th>通道／操作</th>
                <th>結果</th>
                <th>請求編號</th>
                <th>耗時</th>
              </tr>
            </thead>
            <tbody>
              {events.map((e) => (
                <tr key={e.id}>
                  <td>{utc(e.at)}</td>
                  <td>
                    {e.transport} / {e.operation}
                  </td>
                  <td>
                    {e.outcome} · {e.status}
                  </td>
                  <td>
                    <code>{e.request_id}</code>
                  </td>
                  <td>{e.duration_ms} ms</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {cursor && (
          <button
            className="btn btn-secondary"
            disabled={loadingPage}
            onClick={() => void loadEarlier()}
          >
            載入較早紀錄
          </button>
        )}
      </section>
      {user.role === "admin" && (
        <section className="access-section">
          <header>
            <span>05 / REGISTERED CLIENTS</span>
            <h2>註冊應用程式</h2>
          </header>
          <form
            className="access-client-form"
            onSubmit={(e) => {
              e.preventDefault();
              void action(async () => {
                await accessRequest("/clients", "POST", {
                  client_name: clientName.trim(),
                  redirect_uris: redirect
                    .split("\n")
                    .map((v) => v.trim())
                    .filter(Boolean),
                  token_endpoint_auth_method: "none",
                  grant_types: ["authorization_code", "refresh_token"],
                  response_types: ["code"],
                });
                setClientName("");
                setRedirect("");
                await reload();
              });
            }}
          >
            <label>
              應用名稱
              <input
                className="input"
                value={clientName}
                onChange={(e) => setClientName(e.target.value)}
                maxLength={80}
                required
              />
            </label>
            <label>
              回呼網址（每行一個 HTTPS 或 loopback URI）
              <textarea
                className="input"
                value={redirect}
                onChange={(e) => setRedirect(e.target.value)}
                required
              />
            </label>
            <button className="btn btn-primary" disabled={busy}>
              註冊應用
            </button>
          </form>
          {clients.map((c) => (
            <article className="access-record" key={c.client_id}>
              <h3>{c.client_name}</h3>
              <code>{c.client_id}</code>
              <ul>
                {c.redirect_uris.map((uri) => (
                  <li key={uri}>
                    <code>{uri}</code>
                  </li>
                ))}
              </ul>
            </article>
          ))}
        </section>
      )}
    </div>
  );
}
