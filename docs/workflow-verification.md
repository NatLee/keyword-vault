# 封面與實際操作驗證

驗證日期：2026-11-27。本次只改 README、圖片、捕圖腳本與說明，未改產品程式碼。

## 圖片來源

- `images/cover.png` 是獨立的 1440 × 720 品牌插畫，可編輯來源為
  [images/src/cover.html](images/src/cover.html)，沿用專案既有 Noto Serif TC 的
  Latin 字型子集與 [OFL 授權](fonts/OFL-Noto-Serif-TC.txt)。這是 README 封面，
  不代表已設定 GitHub Social Preview。
- 三張 `workflow-*.png` 都是實際 1440 × 1100 瀏覽器 viewport 擷取。
  第一步是完整語法庫的上方；第二步捲到「從 #1 載入」的草稿／結構工作區；
  第三步捲到私人副本的儲存結果。沒有合成控制項或成功狀態。
- 使用內建虛構 demo：#1「珍珠奶茶」→ 手搖飲分冊 → 同一筆語法的試寫副本。
  demo 人名是範例資料，不是真實個資。原本的首頁等畫面仍保留在 README 圖庫。

## 重現

先依 README 建立、編譯一個**隔離、已載入 demo 的執行個體**。請勿使用正式資料庫。
在新資料目錄可執行以下命令載入內建範例，記下它顯示的 demo 密碼，再啟動伺服器：

```sh
cd src/api
KV_DATA_DIR=/tmp/keyword-vault-workflow-demo cargo run -- demo
KV_DATA_DIR=/tmp/keyword-vault-workflow-demo KV_BIND=127.0.0.1:18686 KV_STATIC_DIR=../web/dist cargo run
```

前端須先依 README 使用 `pnpm build` 產生 `src/web/dist`。也可使用開發伺服器 URL。
另開終端機，在 repo 根目錄執行，將密碼只放在環境變數：

```sh
python -m pip install playwright
python -m playwright install chromium
KV_QA_URL=http://127.0.0.1:18686 KV_QA_PASSWORD='<demo password>' python docs/capture-workflow.py
# 如果使用系統 Chrome，另設 PLAYWRIGHT_CHANNEL=chrome
```

腳本經實際登入、點擊「手搖飲」分冊與 #1 的「拆解與試寫」入口，改寫為
`(珍珠奶茶|波霸奶茶)&評價!(業配|廣告)`，選取廣告節點，然後以
「珍珠奶茶 · 去除廣告的口碑草稿」另存私人語法。同時重新產生封面。

## 實際檢查結果

- `PLAYWRIGHT_CHANNEL=chrome python docs/capture-workflow.py` — 通過；
  [執行輸出](verification/workflow-browser.log) 不含密碼。
- UI：主題篩選、#1 的入口與來源標示、草稿內容、排除群組節點、編譯輸出與儲存成功連結都符合。
- 資料：直接讀取 authenticated API，確認新副本的 expression、private 範圍與來源標籤；
  完整比對 #1 原稿未變。用 `finally` 刪除臨時副本，確認回應 404 且總筆數回到起始值。
- 真實 Rust `/api/tools/check` 的 `query_string` 與畫面相同；不另以 JavaScript 推測編譯結果。
- 390 px 下同一草稿與新副本詳情均無 document 橫向溢位；鍵盤 Enter 能選取節點。
- 無 browser page errors；截圖前等待本地字型與兩次 animation frame。
- `python -m py_compile docs/capture-workflow.py` 與 `git diff --check` 通過。
- README 本地連結／圖片存在，docs PNG 全部可解碼，封面尺寸確為 1440 × 720。
- 已逐張目視檢查四個流程／封面資產（含 720 px 封面縮圖）；未發現需改產品行為的缺口。

本次使用既有虛構 demo 資料庫的 SQLite backup，啟動在隔離目錄與 18686 port；
沒有變更原 demo 資料庫、發送外部通知或連接搜尋引擎。檢查證明編譯與儲存流程，
不代表特定搜尋引擎上的命中結果。
