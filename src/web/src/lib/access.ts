/** Access DTOs never include provider tokens or credential hashes. */
export type Provider = {
  provider: string;
  name: string;
  enabled: boolean;
  login_start_url: string;
  disabled_reason: string | null;
};
export type Identity = {
  id: string;
  provider: string;
  subject_mask: string;
  linked_at: string;
  last_login_at: string | null;
};
export type Credential = {
  id: string;
  name: string;
  prefix: string;
  scopes: string[];
  audiences: string[];
  created_at: string;
  expires_at: string;
  last_used_at: string | null;
  revoked_at: string | null;
};
export type Grant = {
  id: string;
  client_id: string;
  client_name: string;
  scopes: string[];
  resource: string;
  created_at: string;
  expires_at: string;
  last_used_at: string | null;
  revoked_at: string | null;
};
export type Counts = {
  attempts: number;
  successes: number;
  failures: number;
  pending: number;
};
export type Usage = {
  days: number;
  from: string;
  to: string;
  timezone: string;
  transport: string | null;
  totals: Counts;
  daily: ({ date: string } & Counts)[];
};
export type AccessEvent = {
  id: string;
  request_id: string;
  at: string;
  kind: string;
  transport: string;
  operation: string;
  user_id: number | null;
  client_id: string | null;
  credential_id: string | null;
  status: number;
  outcome: string;
  duration_ms: number;
};
export type Overview = {
  identities: Identity[];
  credentials: Credential[];
  grants: Grant[];
  summary: Counts & {
    days: number;
    from: string;
    to: string;
    timezone: string;
  };
};
export type Client = {
  client_id: string;
  client_name: string;
  redirect_uris: string[];
  created_at: string;
};
export type Consent = {
  request_id: string;
  client_name: string;
  scopes: string[];
  resource: string;
  expires_at: string;
  user: { id: number; username: string; display_name: string };
};
export async function accessRequest<T>(
  path: string,
  method = "GET",
  body?: unknown,
): Promise<T> {
  const response = await fetch(`/api/access${path}`, {
    method,
    credentials: "same-origin",
    headers: body === undefined ? {} : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
    cache: "no-store",
  });
  if (response.status === 204) return undefined as T;
  const data = await response.json();
  if (!response.ok)
    throw new Error(
      typeof data.error === "string"
        ? data.error
        : `伺服器回應 ${response.status}`,
    );
  return data as T;
}
export function authorizeNext(search = window.location.search): string | null {
  const next = new URLSearchParams(search).get("next");
  if (
    !next ||
    !next.startsWith("/oauth/authorize?") ||
    next.includes("\\") ||
    next.includes("#") ||
    /[\u0000-\u001f]/.test(next)
  )
    return null;
  try {
    const url = new URL(next, window.location.origin);
    return url.origin === window.location.origin &&
      url.pathname === "/oauth/authorize" &&
      url.search
      ? url.pathname + url.search
      : null;
  } catch {
    return null;
  }
}
const providerErrors: Record<string, string> = {
  account_pending: "帳號等待管理員審核，核准後才能登入。",
  account_rejected: "帳號申請未獲核准，請聯絡管理員。",
  registration_closed: "目前未開放註冊，請聯絡管理員。",
  setup_required: "請先建立第一位管理員。",
  identity_not_linked:
    "這個外部帳號尚未綁定。請先以原帳號登入，再到連線與存取綁定。",
  identity_conflict: "這個外部帳號已綁定其他帳號。",
  state_invalid: "登入或綁定請求已過期，或不屬於目前瀏覽器。請重新開始。",
  provider_unavailable: "供應商暫時無法連線；仍可使用原帳號登入。",
  access_denied: "已取消第三方授權。",
  account_unavailable: "這個帳號目前無法使用，請聯絡管理員。",
  provider_disabled: "尚未設定這個登入供應商；請使用原帳號登入。",
};
export function providerError(search = window.location.search): string | null {
  const code = new URLSearchParams(search).get("access_error");
  return code && Object.hasOwn(providerErrors, code)
    ? providerErrors[code]
    : null;
}
export const SCOPE_LABEL: Record<string, string> = {
  "keywords:read": "語法讀取",
  "expressions:read": "語法檢查與比對",
  "queries:read": "編譯器相容 API",
};
export const utc = (value: string | null) =>
  value ? value.replace("T", " ").replace(/\.\d+Z$/, "Z") : "尚未使用";
