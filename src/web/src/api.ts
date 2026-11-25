import type {
  Account,
  AuthState,
  CheckResult,
  Comment,
  DiffResult,
  Group,
  HistoryEntry,
  Keyword,
  KeywordDetail,
  KeywordInput,
  KeywordPage,
  ListFilter,
  Meta,
  Role,
  Stats,
  TagCount,
  User,
} from "./types";

/** session 過期或沒登入會收到 401；App 聽到這個事件就回到登入頁。 */
export const UNAUTHORIZED_EVENT = "kv:unauthorized";

/** 後端回 `{"error": "訊息"}` 時丟出這個，訊息直接給使用者看。 */
export class ApiError extends Error {
  readonly status: number;
  /** 完整的回應內容（匯入失敗時裡面有每一列的問題） */
  readonly data: unknown;
  constructor(status: number, message: string, data: unknown) {
    super(message);
    this.status = status;
    this.data = data;
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const init: RequestInit = { method, headers: {} };
  if (body instanceof Blob) {
    init.body = body;
    (init.headers as Record<string, string>)["content-type"] = "text/csv";
  } else if (body !== undefined) {
    init.body = JSON.stringify(body);
    (init.headers as Record<string, string>)["content-type"] = "application/json";
  }
  const res = await fetch(`/api${path}`, init);
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  let data: unknown = undefined;
  try {
    data = text ? JSON.parse(text) : undefined;
  } catch {
    data = text;
  }
  if (res.status === 401 && !path.startsWith("/auth/")) {
    window.dispatchEvent(new Event(UNAUTHORIZED_EVENT));
  }
  if (!res.ok) {
    const message =
      typeof data === "object" && data !== null && "error" in data
        ? String((data as { error: unknown }).error)
        : `伺服器回應 ${res.status}`;
    throw new ApiError(res.status, message, data);
  }
  return data as T;
}

const get = <T>(path: string) => request<T>("GET", path);
const post = <T>(path: string, body?: unknown) => request<T>("POST", path, body ?? {});
const patch = <T>(path: string, body: unknown) => request<T>("PATCH", path, body);
const del = <T = void>(path: string) => request<T>("DELETE", path);

function query(params: object): string {
  const q = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== null && v !== "") q.set(k, String(v));
  }
  const s = q.toString();
  return s ? `?${s}` : "";
}

export const api = {
  meta: () => get<Meta>("/meta"),

  authState: () => get<AuthState>("/auth/state"),
  // 顯示名稱沒填就不送，後端會用帳號當名字
  setup: (input: { username: string; display_name: string; password: string }) =>
    post<User>("/auth/setup", { ...input, display_name: input.display_name || undefined }),
  login: (username: string, password: string) => post<User>("/auth/login", { username, password }),
  logout: () => post<void>("/auth/logout"),
  updateMe: (displayName: string) => patch<User>("/auth/me", { display_name: displayName }),
  changePassword: (currentPassword: string, newPassword: string) =>
    post<void>("/auth/password", { current_password: currentPassword, new_password: newPassword }),

  users: () => get<Account[]>("/users"),
  createUser: (input: { username: string; display_name: string; password: string; role: Role }) =>
    post<Account>("/users", { ...input, display_name: input.display_name || undefined }),
  updateUser: (id: number, input: { display_name?: string; role?: Role; disabled?: boolean; password?: string }) =>
    patch<Account>(`/users/${id}`, input),

  groups: () => get<Group[]>("/groups"),
  createGroup: (name: string, memberIds: number[]) => post<Group>("/groups", { name, member_ids: memberIds }),
  updateGroup: (id: number, input: { name?: string; member_ids?: number[] }) => patch<Group>(`/groups/${id}`, input),
  deleteGroup: (id: number) => del(`/groups/${id}`),

  keywords: (filter: ListFilter = {}) => get<KeywordPage>(`/keywords${query(filter)}`),
  keyword: (id: number) => get<KeywordDetail>(`/keywords/${id}`),
  randomKeyword: () => get<Keyword | null>("/keywords/random"),
  createKeyword: (input: KeywordInput) => post<Keyword>("/keywords", input),
  updateKeyword: (id: number, input: KeywordInput) =>
    patch<{ keyword: Keyword; cascaded: number }>(`/keywords/${id}`, input),
  deleteKeyword: (id: number, cascade: boolean) =>
    del<{ deleted: number }>(`/keywords/${id}${query({ cascade: cascade || undefined })}`),
  history: (id: number) => get<HistoryEntry[]>(`/keywords/${id}/history`),
  setFavorite: (id: number, favorite: boolean) =>
    request<{ favorite: boolean }>(favorite ? "PUT" : "DELETE", `/keywords/${id}/favorite`),
  importCsv: (file: Blob) => request<{ created: number }>("POST", "/keywords/import", file),
  exportUrl: (filter: ListFilter) => `/api/keywords/export${query({ ...filter, page: undefined, page_size: undefined })}`,

  comments: (keywordId: number) => get<Comment[]>(`/keywords/${keywordId}/comments`),
  addComment: (keywordId: number, body: string) => post<Comment>(`/keywords/${keywordId}/comments`, { body }),
  updateComment: (id: number, body: string) => patch<Comment>(`/comments/${id}`, { body }),
  deleteComment: (id: number) => del(`/comments/${id}`),

  tags: () => get<TagCount[]>("/tags"),
  stats: () => get<Stats>("/stats"),

  check: (expression: string, precise = true) => post<CheckResult>("/tools/check", { expression, precise }),
  diff: (a: string, b: string) => post<DiffResult>("/tools/diff", { a, b }),
};
