import { useEffect, useState } from "react";
import { useSearchParams } from "react-router";
import {
  accessRequest,
  SCOPE_LABEL,
  type Consent as PendingConsent,
  utc,
} from "../lib/access";
import { ErrorBanner } from "../components/ui";
export function Consent() {
  const [params] = useSearchParams();
  const id = params.get("request_id");
  const [data, setData] = useState<PendingConsent | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    setData(null);
    if (!id) {
      setError("缺少授權請求編號。");
      return;
    }
    accessRequest<PendingConsent>(`/consents/${encodeURIComponent(id)}`)
      .then((v) => {
        if (active) setData(v);
      })
      .catch((e) => {
        if (active) setError(String(e.message));
      });
    return () => {
      active = false;
    };
  }, [id]);
  return (
    <section className="access-consent">
      <p className="eyebrow">APPLICATION REQUEST / 授權申請單</p>
      <h1>應用程式授權</h1>
      {error && <ErrorBanner message={error} />}{" "}
      {!data && !error && <p role="status">讀取授權申請…</p>}
      {data && (
        <>
          <h2>{data.client_name}</h2>
          <p>
            目前帳號：{data.user.display_name}（{data.user.username}）
          </p>
          <dl>
            <dt>請求權限</dt>
            <dd>
              <ul>
                {data.scopes.map((scope) => (
                  <li key={scope}>
                    {SCOPE_LABEL[scope] ?? scope} <code>{scope}</code>
                  </li>
                ))}
              </ul>
            </dd>
            <dt>指定資源</dt>
            <dd>
              <code>{data.resource}</code>
            </dd>
            <dt>申請期限（UTC）</dt>
            <dd>{utc(data.expires_at)}</dd>
          </dl>
          <p>
            授權範圍由伺服器保留；應用仍只能讀取此帳號目前可見的語法。可隨時在連線與存取撤銷。
          </p>
          <form
            method="post"
            action={`/oauth/consent/${encodeURIComponent(data.request_id)}`}
          >
            <button className="btn btn-primary" name="decision" value="approve">
              同意授權
            </button>
            <button className="btn btn-secondary" name="decision" value="deny">
              拒絕
            </button>
          </form>
        </>
      )}
    </section>
  );
}
