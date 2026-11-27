# 編譯工作台與中文斷詞

## 函式庫、CLI 與工作台

單一 workspace 的 `crates/es-keyword-query` 保留原本套件名稱、0.1.0 API、Rust 1.74 MSRV 和 `unicode-normalization` 輕量 runtime dependency；jieba、HTTP、資料庫與帳號套件只在 host。核心可獨立複製／使用 path dependency。CLI 與固定 atlas 在同個 repo 維護：

```bash
cargo run -p es-keyword-query --example inspect -- '茶&奶'
cargo run -p es-keyword-query --example inspect -- '(茶 奶 ~)' --raw --slop=8
cargo run -p es-keyword-query --example build_atlas -- --check
# Rust 1.74 環境，將核心複製到 workspace 外測試：
bash scripts/check-compiler-msrv.sh
```

語法解剖的精確／raw、預設 slop 和 optional filter 均是明確的預覽設定。篩選使用 parser 的 `filtered_by`，兩側都分組。另存來源語法只存來源；先按「套用篩選至草稿」才會把括號與 `&` 寫入來源編輯區。匯出 JSON 包含 schema_version、原始來源、filter、effective options、query／filter 的 AST／tree 與最終 query string。載入不同語法時不讓較舊的非同步回應覆蓋新預覽。

## HTTP、API 與 MCP

Cookie session 使用 `POST /api/tools/inspect`。Body：

```json
{"expression":"茶|奶","filter_expression":"台灣|臺灣","precise":true,"default_slop":8}
```

未填 `precise` 為 true，未填 slop 使用伺服器 `KV_DEFAULT_SLOP`；slop 必須是 u32。來源與 filter 合計上限50,000 Unicode scalars。成功為 `{valid:true,inspection:{schema_version:1,expression,filter_expression,options,query,filter,query_string}}`；語法失敗維持200與 `{valid:false,error:{source,message,position}}`，source指出來源或filter。未知欄位拒絕。

`POST /api/v1/inspect_workbench` 與 MCP `inspect_workbench` 使用相同 schema，scope `expressions:read`。既有 `inspect_expression` 的行為、body、輸出與50,000字限制不變。

舊版相容操作為 `compile_query`、`inspect_query`、`extract_terms`，在 `/api/v1/{operation}` 與 MCP 使用獨立 `queries:read` scope。它不會自動加到舊憑證，也不預選於新金鑰表單。Body為 `{expression,precise?,default_slop?}`，上限40,000 Unicode scalars，保留原 host 的英語400語法錯誤、`compile_error` 機器代碼及輸出形狀；MCP使用相同代碼與英語訊息的 structured error。舊 Cookie `/api/compiler/inspect` 仍可用。所有路徑使用同一個核心；沿用 bearer audience、scope、停用／審核狀態與單次計次。用 `/api/v1/openapi.json` 或 MCP tools/list 取得實際完整 schemas。

## 繁體中文預覽

`POST /api/tools/analyze`（Cookie）、`POST /api/v1/analyze_expression`、MCP `analyze_expression`（`expressions:read`）：

```json
{"expression":"珍珠奶茶!廣告","mode":"normal","precise":true}
```

`mode` 為 normal（預設）或 search。只分析已經由 parser 解析的 term／proximity 葉節點，保留 exclusion 標記；raw mode 與未解析的 `#123` 引用拒絕。輸入最多10,000 Unicode scalars、輸出最多20,000 tokens，超過會報錯、不截斷；每個服務最多兩個並行分析，忙碌回429。取消請求仍保留實際工作佔用的名額直到完成。

固定 `jieba-rs 0.11.0`、HMM=false，詞典為 pinned `dict.txt.big` 加上台灣領域詞（來源及 MIT license 在 `data/segmentation`）。回應包含 engine／engine_version、mode、effective dictionary ID／SHA-256、`offset_unit: "unicode_scalar"`，每個葉節點的 clause/text/negated/tokens。token start/end 是該**解析後葉節點文字**的 Unicode scalar半開區間，非原始表達式 bytes，也非 JavaScript UTF-16 indices。複製單一 token 會跳脫自訂語法保留字，不會自行決定 AND／OR。

需要自訂詞時，在部署端掛載 UTF-8 檔案並設定 `KV_SEGMENTATION_DICT=/app/config/user-dict.txt`，重新啟動服務；API 不接受使用者指定檔案路徑。每行 `詞 頻率 [詞性]`，例如：

```text
台灣高鐵 1000000 nz
光儲整合 1000000 nz
```

單檔最多1 MiB／10,000有效行，詞長1–100 Unicode scalars、頻率1..1,000,000,000；重複詞、錯誤UTF-8、格式或路徑使啟動失敗。內建詞典加自訂檔案的實際 bytes共同決定回應 SHA-256。更換詞典後應重新確認 token；不會自動繁簡轉換。

這是**本機中文斷詞預覽，不代表 Elasticsearch 分析結果**。`query_string` 的引號詞受 Elasticsearch欄位 analyzer／quote_analyzer影響，需確保 index與search設定相容；SmartCN主要處理簡體中文，不能作為繁體詞典的等價替代。本產品不安裝ES插件、不呼叫ES、不重新索引、不改變編譯器布林語意，也不改動 Vault 的 SQLite子字串搜尋。部署到ES前，請以實際index的 `_analyze` 與查詢測試確認。

參考：[Elasticsearch query_string](https://www.elastic.co/docs/reference/query-languages/query-dsl/query-dsl-query-string-query)、[analyze API](https://www.elastic.co/docs/api/doc/elasticsearch/operation/operation-indices-analyze)、[SmartCN](https://www.elastic.co/docs/reference/elasticsearch/plugins/analysis-smartcn)、[jieba-rs](https://github.com/messense/jieba-rs)。
