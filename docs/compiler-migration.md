# 舊編譯器帳號移轉

僅支援舊 `es-keyword-query` host 與本版 Vault 的 SQLx migrations 0001–0004；工具核對每份 migration 的 SHA-384、SQLite integrity／foreign-key check，以及實際 schema／整個檔案的 SHA-256。這是帳號匯入，不是整個資料庫替換，也不是 CSV 匯入。

## 準備

1. 備份兩邊完整資料並停止兩個服務。保持停止直到匯入及驗證完成；不要針對線上資料庫執行。
2. 正常關閉 SQLite 連線。若尚有非空 `-wal`，先在停止服務後用 SQLite `PRAGMA wal_checkpoint(TRUNCATE);` checkpoint，再關閉連線。不要刪除 WAL。工具拒絕尚有 WAL 資料的檔案。
3. 來源與目的都必須是已完成 migrations 的既有、不同資料庫。目的可以已經有帳號、關鍵字與歷史。先升級／初始化目的 Vault，再停止服務；匯入指令不會建立目的資料庫，也不會讀取或初始化 `KV_DATABASE_PATH`。
4. 使用有讀取來源及寫入目的／備份目錄權限的本機帳號。報告包含帳號、provider subject、檔案路徑等管理資料；在 Unix 新報告與備份以 `0600` 建立，請妥善保存。

## 唯讀 dry-run

```bash
keyword-vault import-compiler \
  --source /offline/compiler.db \
  --destination /offline/keyword-vault.db \
  --report /offline/account-import.json
```

預設只有 dry-run。新報告列出原始／目的 user ID、identity ID、registration ID、reviewer ID 對照，以及保留的角色、停用旗標、註冊狀態與密碼登入旗標。不輸出密碼雜湊、session、PAT 或 OAuth token。已有報告檔不覆寫。若有衝突仍寫出報告，但回傳非零狀態。

支援 `admin`／`member`、`active`／`pending`／`rejected`，保留 `disabled` 與 `password_login_enabled`。待審／拒絕帳號必須有一致註冊紀錄；審核者必須能對照至來源帳號。沒有密碼登入的帳號必須有支援的 Google／GitHub identity。啟用密碼登入的帳號只接受 Argon2id v19 PHC，`m=8192..65536` KiB、`t=1..6`、`p=1..4`，輸出16–64 bytes，有 salt 且無額外參數。超出格式需先在舊系統明確重設／修復，工具不會替使用者改密碼或降低成本。

**任何同名帳號（SQLite ASCII NOCASE 規則）、相同 provider+subject、未知狀態／雜湊、缺少審核者或不一致註冊資料都阻止整批匯入。** 不會依 username、email、provider subject 自動合併身分，也不能手改報告指定別人的 ID。請由管理員確認並在來源／目的系統解決衝突，再產生新的報告。保留原始備份與明確的處理紀錄。

## 明確 apply

確認報告的兩個路徑、指紋、對照與空的 `conflicts`，使用全新備份檔名：

```bash
keyword-vault import-compiler \
  --apply /offline/account-import.json \
  --backup /offline/keyword-vault.before-import.db
```

工具重新產生完整報告並比較，鎖定目的資料庫後再驗證；任何資料／schema／對照改動都要求重新 dry-run。先建立全新目的備份，驗證 bytes SHA-256、schema 與 integrity，之後才在單一 transaction 插入帳號、identity 與註冊狀態。任何插入失敗會回復整批寫入，已驗證的備份留存供檢查；不會覆寫既有備份。成功回傳 ID 對照與備份指紋。再套用同一報告會因目的指紋改變而拒絕。

來源檔案保持不變。Vault 既有帳號、owner/group visibility、關鍵字、標籤、引用、版本與 CSV 行為保持不變。舊 sessions、PAT、OAuth clients／consents／grants／codes／tokens、provider flows、使用量／稽核、註冊事件與註冊 policy 不匯入；報告列出排除筆數，原始資料在舊備份中保留。目的 policy 原樣保留。舊服務無關鍵字資料可匯入；需要搬移 Vault 資料時應走獨立的完整備份復原流程。

啟動 Vault 後驗證管理員、一般成員、停用／待審／拒絕與 OAuth-only 帳號限制。用戶須重新登入，OAuth provider callback 改為新的 Vault origin；外部 OAuth clients 重新登記並重新同意，API/MCP 金鑰重新發行及選擇 scope。完成驗證前勿刪除來源或舊 repo 歷史。

若要回復，停止 Vault，另外保留失敗後資料作調查，再使用已驗證的備份復原目的資料目錄。確保沒有任何舊 WAL／SHM 混用；不要在線上替換單一 DB 檔案。這是管理員明確的離線復原操作，匯入工具不會自動覆蓋現有資料。
