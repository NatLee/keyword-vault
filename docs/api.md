# API

所有路徑都在 `/api` 底下，內容是 JSON。錯誤一律是 `{"error": "給人看的訊息"}` 加上對應的狀態碼。

除了 `health`、`meta` 和 `auth/state|setup|login` 之外都要先登入；登入後瀏覽器會帶著 `kv_session` cookie（HttpOnly、SameSite=Lax）。寫入請求（POST、PUT、PATCH、DELETE）如果帶了別的網站的 `Origin` 會回 403。

時間是 RFC 3339 的 UTC，例如 `2026-01-05T08:00:00Z`。

## 狀態

| 方法 | 路徑 | 說明 |
|---|---|---|
| GET | `/health` | 回 `ok` |
| GET | `/meta` | `{version, default_slop}` |

## 登入

| 方法 | 路徑 | 內容 | 回應 |
|---|---|---|---|
| GET | `/auth/state` | | `{setup_required, user}`，沒登入時 `user` 是 `null` |
| POST | `/auth/setup` | `{username, display_name?, password}` | 建立第一個管理員並登入；已經有帳號時不能用 |
| POST | `/auth/login` | `{username, password}` | `User` |
| POST | `/auth/logout` | | |
| PATCH | `/auth/me` | `{display_name}` | `User` |
| POST | `/auth/password` | `{current_password, new_password}` | 其他裝置的登入會失效 |

`User` 是 `{id, username, display_name, role}`，`role` 是 `admin` 或 `member`。帳號 3–32 個字元（`A-Z a-z 0-9 _ . -`），密碼至少 8 個字元，顯示名稱 1–40 個字元。

## 帳號和群組

帳號只有管理員能用。

| 方法 | 路徑 | 內容 | 回應 |
|---|---|---|---|
| GET | `/users` | | `Account[]` |
| POST | `/users` | `{username, display_name?, password, role?}` | `Account` |
| PATCH | `/users/{id}` | `{display_name?, role?, disabled?, password?}` | `Account`。不能改自己的角色或停用自己 |
| GET | `/groups` | | `Group[]`。管理員看到全部，其他人只看到自己所在的 |
| POST | `/groups` | `{name, member_ids?}` | 管理員 |
| PATCH | `/groups/{id}` | `{name?, member_ids?}` | 管理員。`member_ids` 是整份名單 |
| DELETE | `/groups/{id}` | | 管理員 |

`Account` 是 `User` 加上 `{disabled, created_at, keyword_count}`。`Group` 是 `{id, name, members: [{id, username, display_name}]}`。帳號不能刪除，只能停用：停用後馬上被登出，寫過的關鍵字會留著。

## 關鍵字

| 方法 | 路徑 | 說明 |
|---|---|---|
| GET | `/keywords` | 清單，見下面的查詢參數。回 `{items, total, page, page_size}` |
| POST | `/keywords` | 新增。一般關鍵字給 `expression`，組合關鍵字給 `logic` |
| GET | `/keywords/random` | 隨機一個別人看得到的關鍵字，沒有時是 `null` |
| GET | `/keywords/{id}` | `KeywordDetail` |
| PATCH | `/keywords/{id}` | 修改，回 `{keyword, cascaded}`；`cascaded` 是跟著更新的組合關鍵字數量 |
| DELETE | `/keywords/{id}` | 有組合關鍵字用到它時回 409；加 `?cascade=true` 連它們一起刪。回 `{deleted}` |
| GET | `/keywords/{id}/history` | `[{id, expression, user_name, created_at}]`，新的在前面；`expression` 是那次修改**之前**的語法 |
| PUT / DELETE | `/keywords/{id}/favorite` | 收藏 / 取消收藏，回 `{favorite}` |
| POST | `/keywords/import` | 內容是 CSV 本身（`content-type: text/csv`）。成功回 `{created}`；有問題回 400 和 `{error, rows: [{line, theme, message}]}`，一筆都不會寫入 |
| GET | `/keywords/export` | CSV，接受和清單一樣的篩選參數 |
| GET | `/tags` | `[{name, count}]`，只算看得到的關鍵字 |
| GET | `/stats` | 總覽頁用的數字 |

清單的查詢參數：

| 參數 | 值 |
|---|---|
| `q` | 搜尋主題和語法 |
| `tag` | 標籤 |
| `scope` | `all`（預設）、`mine`、`shared`（別人分享的）、`favorites` |
| `kind` | `plain`、`combination` |
| `sort` | `updated`（預設）、`created`、`theme` |
| `page`、`page_size` | 從 1 開始 |

新增和修改的內容：

```json
{
  "theme": "手搖飲",
  "expression": "珍珠奶茶|(珍珠 鮮奶 ~)",
  "visibility": "public",
  "tags": ["飲料", "熱門"]
}
```

組合關鍵字把 `expression` 換成 `"logic": "(#3|#5)!#8"`，至少要用到兩個不同、而且自己看得到的關鍵字。`visibility` 是 `private`（預設）、`group` 或 `public`。主題最多 100 個字元；標籤最多 10 個，每個最多 30 個字元。一般關鍵字和組合關鍵字不能互換。

`Keyword`：

```json
{
  "id": 33,
  "theme": "手搖飲聲量（去業配）",
  "expression": "((珍珠奶茶|波霸奶茶)|(手搖飲|手搖杯))!(業配|抽獎)",
  "logic": "(#1|#2)!#3",
  "is_combination": true,
  "visibility": "public",
  "owner": { "id": 1, "username": "demo", "display_name": "林小安" },
  "tags": ["手搖飲", "飲料"],
  "favorite": false,
  "comment_count": 1,
  "created_at": "2026-02-18T05:42:00Z",
  "updated_at": "2026-02-18T05:42:00Z",
  "can_edit": true,
  "can_delete": true
}
```

`KeywordDetail` 多了 `query_string`、`terms`（用到的詞）、`refs`（組合關鍵字引用的關鍵字）和 `dependents`（用到它的組合關鍵字），後兩個是 `[{id, theme}]`；看不到的關鍵字 `theme` 是 `null`。

誰看得到：自己的全部、`public` 的全部、`group` 的要和擁有者在同一個群組。管理員也一樣，看不到別人的 `private`。只有擁有者能修改；擁有者和管理員能刪除（管理員只刪得到自己看得到的）。

## 留言

| 方法 | 路徑 | 內容 |
|---|---|---|
| GET | `/keywords/{id}/comments` | |
| POST | `/keywords/{id}/comments` | `{body}` |
| PATCH | `/comments/{id}` | `{body}`，只有留言的人 |
| DELETE | `/comments/{id}` | 留言的人或管理員 |

`Comment` 是 `{id, body, user_id, user_name, created_at, updated_at, can_edit, can_delete}`。

## 工具

| 方法 | 路徑 | 內容 | 回應 |
|---|---|---|---|
| POST | `/tools/check` | `{expression, precise?}` | `{valid: true, query_string, terms}` 或 `{valid: false, error: {message, position}}`，`position` 從 0 算 |
| POST | `/tools/diff` | `{a, b}` | `{same, segments: [{op, text}]}`，`op` 是 `equal`、`insert`、`delete` |

`precise`（預設 `true`）會把每個詞加上引號當成完整的詞；設成 `false` 時詞照原樣送出，`*` 這類 Lucene 語法才有作用。

```bash
curl -b cookies.txt -H 'content-type: application/json' \
  -d '{"expression":"(珍珠奶茶|(珍珠 鮮奶 ~))&手搖!廣告"}' http://localhost:8686/api/tools/check
```

## 語法結構（解剖工作台）

成功的 `POST /api/tools/check` 與 `GET /api/keywords/:id` 現在另回傳 `clauses`。
這是 Rust 解析樹的前序清單；每個節點包含：

- `parent`：所屬群組在清單的索引，最外層為 `null`。
- `kind`：`term`、`proximity` 或 `group`。
- `joiner`：前接 `AND`、`OR` 或 `null`；`negated` 表示此節點直接有 `!`。
- `text`：正規化後的詞彙／片語；群組為 `null`。
- `slop`：片語實際使用的 slop；其他節點為 `null`。
- `output`：單一詞彙／片語的編譯文字，不含前接運算子和排除；群組為 `null`。

父節點一定先於子節點。排除群組的子節點不重複標記 `negated`。此資料只描述寫入的括號與順序，不加入搜尋引擎的運算子優先順序。無效語法回應不含 `clauses`。
