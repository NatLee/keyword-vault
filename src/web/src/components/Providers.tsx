import { useEffect, useState } from "react";
import { LogIn } from "lucide-react";
import {
  accessRequest,
  authorizeNext,
  providerError,
  type Provider,
} from "../lib/access";
export function Providers() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    accessRequest<{ items: Provider[] }>("/providers")
      .then((v) => {
        if (active) setProviders(v.items);
      })
      .catch(() => {
        if (active)
          setError("第三方登入選項暫時無法載入。仍可使用原帳號登入。");
      });
    return () => {
      active = false;
    };
  }, []);
  return (
    <div className="provider-login">
      {providerError() && <p role="status">{providerError()}</p>}
      <p>已綁定帳號的第三方登入</p>
      {providers.map((p) => (
        <div key={p.provider}>
          <button
            type="button"
            className="btn btn-secondary"
            disabled={!p.enabled}
            title={p.disabled_reason ?? undefined}
            onClick={() => {
              const next = authorizeNext();
              window.location.assign(
                p.login_start_url +
                  (next ? `?next=${encodeURIComponent(next)}` : ""),
              );
            }}
          >
            <LogIn className="size-4" />
            {p.name} 登入
          </button>
          {!p.enabled && <small>{p.name} 尚未設定；請管理員配置供應商。</small>}
        </div>
      ))}
      {error && <p role="status">{error}</p>}
      <small>未綁定時，先使用原帳號登入，再到「連線與存取」綁定。</small>
    </div>
  );
}
