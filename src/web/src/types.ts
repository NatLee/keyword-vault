export type Role = "admin" | "member";
export type Visibility = "private" | "group" | "public";

export interface Meta {
  version: string;
  /** `~` 後面沒寫數字時用的距離 */
  default_slop: number;
}

export interface User {
  id: number;
  username: string;
  display_name: string;
  role: Role;
}

export interface AuthState {
  /** 還沒有任何帳號：第一個進來的人要先建立管理員 */
  setup_required: boolean;
  user: User | null;
}

export interface Account extends User {
  account_status?: "active" | "pending" | "rejected";
  disabled: boolean;
  created_at: string;
  keyword_count: number;
}

export interface Member {
  id: number;
  username: string;
  display_name: string;
}

export interface Group {
  id: number;
  name: string;
  members: Member[];
}

export interface Keyword {
  id: number;
  theme: string;
  expression: string;
  /** 組合關鍵字的組合方式，例如 `(#3|#5)!#8`；一般關鍵字是 null */
  logic: string | null;
  is_combination: boolean;
  visibility: Visibility;
  owner: Member;
  tags: string[];
  favorite: boolean;
  comment_count: number;
  created_at: string;
  updated_at: string;
  can_edit: boolean;
  can_delete: boolean;
}

export interface Linked {
  id: number;
  /** 看不到的關鍵字沒有主題 */
  theme: string | null;
}

export interface KeywordDetail extends Keyword {
  query_string: string;
  terms: string[];
  clauses: Clause[];
  refs: Linked[];
  dependents: Linked[];
}

export interface KeywordPage {
  items: Keyword[];
  total: number;
  page: number;
  page_size: number;
}

export type Scope = "all" | "mine" | "shared" | "favorites";
export type Kind = "plain" | "combination";
export type Sort = "updated" | "created" | "theme";

export interface ListFilter {
  q?: string;
  tag?: string;
  scope?: Scope;
  kind?: Kind;
  sort?: Sort;
  page?: number;
  page_size?: number;
}

export interface KeywordInput {
  theme?: string;
  expression?: string;
  logic?: string;
  visibility?: Visibility;
  tags?: string[];
}

export interface HistoryEntry {
  id: number;
  /** 修改之前的語法 */
  expression: string;
  user_name: string | null;
  created_at: string;
}

export interface Comment {
  id: number;
  body: string;
  user_id: number;
  user_name: string;
  created_at: string;
  updated_at: string;
  can_edit: boolean;
  can_delete: boolean;
}

export interface TagCount {
  name: string;
  count: number;
}

export interface Stats {
  total: number;
  mine: number;
  mine_private: number;
  mine_shared: number;
  combinations: number;
  favorites: number;
  monthly: { month: string; count: number }[];
  contributors: { name: string; count: number }[];
  recent: Keyword[];
  today: Keyword | null;
}

export interface Clause {
  parent: number | null;
  kind: "term" | "proximity" | "group";
  joiner: "AND" | "OR" | null;
  negated: boolean;
  text: string | null;
  slop: number | null;
  output: string | null;
}

export type CheckResult =
  | { valid: true; query_string: string; terms: string[]; clauses: Clause[] }
  | { valid: false; error: { message: string; position: number } };

export interface DiffSegment {
  op: "equal" | "insert" | "delete";
  text: string;
}

export interface DiffResult {
  same: boolean;
  segments: DiffSegment[];
}

export interface ImportProblem {
  line: number;
  theme: string;
  message: string;
}
