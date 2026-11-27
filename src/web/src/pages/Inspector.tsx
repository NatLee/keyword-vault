import {
  ArrowRight,
  BookOpen,
  FlaskConical,
  RotateCcw,
  Save,
  Search,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Link, useSearchParams } from "react-router";
import { api } from "../api";
import { PageBody } from "../components/Layout";
import { QueryAnatomy } from "../components/QueryAnatomy";
import { CheckResultView, useCheck } from "../components/SyntaxCheck";
import { ErrorBanner, Spinner } from "../components/ui";
import { errorText, useAsync, useDebounced } from "../lib/hooks";
import type { Keyword } from "../types";

const INITIAL = "(茶|咖啡)&評價!(業配|廣告)";
export function Inspector() {
  const [params, setParams] = useSearchParams();
  const id = Number(params.get("keyword")) || null;
  const [search, setSearch] = useState("");
  const query = useDebounced(search, 250);
  const library = useAsync(
    () => api.keywords({ q: query, page_size: 8 }),
    [query],
  );
  const [source, setSource] = useState<Keyword | null>(null);
  const [expression, setExpression] = useState(INITIAL);
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState<number | null>(null);
  const check = useCheck(expression);
  const draft = useRef(expression);
  draft.current = expression;
  const sourceId = useRef(id);
  sourceId.current = id;

  useEffect(() => {
    let active = true;
    setFailure(null);
    setSaved(null);
    setSource(null);
    if (!id) {
      setLoading(false);
      setExpression(INITIAL);
      setName("");
      return;
    }
    setLoading(true);
    setExpression("");
    api
      .keyword(id)
      .then((k) => {
        if (!active) return;
        setSource(k);
        setExpression(k.expression);
        setName(`${k.theme} · 新版本`);
      })
      .catch((e) => {
        if (active) setFailure(errorText(e));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [id]);

  const baseline = source?.expression ?? INITIAL;
  const dirty = expression !== baseline;
  const choose = (next: number | null) => {
    if (next === id) return;
    if (dirty && !window.confirm("切換語法會捨棄尚未儲存的草稿，繼續嗎？"))
      return;
    setParams(next ? { keyword: String(next) } : {});
  };
  const save = async () => {
    if (
      !check.result?.valid ||
      check.pending ||
      busy ||
      !name.trim() ||
      Array.from(expression).length > 5000
    )
      return;
    const submitted = expression,
      selectedId = id;
    setBusy(true);
    setFailure(null);
    try {
      const result = await api.createKeyword({
        theme: name.trim(),
        expression: submitted.trim(),
        visibility: "private",
        tags: source?.tags ?? [],
      });
      if (draft.current === submitted && sourceId.current === selectedId)
        setSaved(result.id);
    } catch (e) {
      setFailure(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <PageBody>
      <div className="inspection-title">
        <div>
          <span className="eyebrow">
            THE ANNOTATION DESK / 典藏・拆解・推敲
          </span>
          <h1>
            把語法拆開，<em>看懂每一層。</em>
          </h1>
          <p>從典藏索引抽出一份語法，在這張推敲桌上留下新想法。</p>
        </div>
        <div className="inspection-stamp">
          <FlaskConical />
          <span>
            ANNOTATION
            <br />
            WORKING COPY
          </span>
        </div>
      </div>
      <div className="inspection-workspace">
        <aside className="inspection-library">
          <div className="inspection-section">
            <span>01 / 選一個起點</span>
            <BookOpen className="size-4" />
          </div>
          <label className="inspection-search">
            <Search className="size-4" />
            <input
              aria-label="搜尋已儲存語法"
              placeholder="找主題或語法…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </label>
          <button
            className={`inspection-new ${id === null ? "selected" : ""}`}
            onClick={() => choose(null)}
          >
            ＋ 使用練習語法 <small>開啟一張乾淨的草稿</small>
          </button>
          {library.error && <ErrorBanner message={library.error} />}
          {library.loading && <p className="inspection-empty">載入語法庫…</p>}
          {library.data?.items.length === 0 && (
            <p className="inspection-empty">找不到符合的語法。</p>
          )}
          <div className="inspection-saved">
            {library.data?.items.map((k) => (
              <button
                key={k.id}
                aria-pressed={id === k.id}
                onClick={() => choose(k.id)}
              >
                <span>#{String(k.id).padStart(3, "0")}</span>
                <strong>{k.theme}</strong>
                <small>{k.expression}</small>
              </button>
            ))}
          </div>
          <p className="inspection-library-note">
            顯示你有權限讀取的語法，最多 8 筆。用搜尋縮小範圍。
            <Link to="/keywords">瀏覽完整語法庫 →</Link>
          </p>
        </aside>
        <div className="inspection-main">
          <div className="inspection-draft">
            <div className="inspection-section">
              <span>02 / 推敲草稿</span>
              <span>{source ? `從 #${source.id} 載入` : "自由練習"}</span>
            </div>
            {loading ? (
              <p className="flex items-center gap-2 py-8">
                <Spinner /> 載入語法…
              </p>
            ) : (
              <>
                <label className="sr-only" htmlFor="inspection-expression">
                  草稿語法
                </label>
                <textarea
                  id="inspection-expression"
                  spellCheck={false}
                  value={expression}
                  onChange={(e) => {
                    setExpression(e.target.value);
                    setSaved(null);
                  }}
                  placeholder="(茶|咖啡)&評價!業配"
                />
                <div className="inspection-draft-footer">
                  <span>
                    {Array.from(expression).length.toLocaleString()} 字元{" "}
                    <b>·</b> {dirty ? "草稿已修改" : "與起點相同"}
                  </span>
                  <button
                    className="btn btn-ghost text-xs"
                    onClick={() => {
                      setExpression(baseline);
                      setSaved(null);
                    }}
                    disabled={!dirty || busy}
                  >
                    <RotateCcw className="size-3" /> 重設草稿
                  </button>
                </div>
              </>
            )}
            {source?.is_combination && (
              <p className="hint">
                載入的是組合展開後的語法；另存為一般語法，不會建立相依關係。
              </p>
            )}
            {source && (
              <Link
                className="inspection-source-link"
                to={`/keywords/${source.id}`}
              >
                檢視原始語法與修改紀錄 <ArrowRight className="size-3" />
              </Link>
            )}
          </div>
          <div className="inspection-result">
            <CheckResultView {...check} />
            {!expression.trim() && !loading && (
              <p className="hint">輸入語法，查看群組、排除條件與編譯結果。</p>
            )}
          </div>
          {check.result?.valid && (
            <QueryAnatomy key={expression} clauses={check.result.clauses} />
          )}
          {failure && <ErrorBanner message={failure} />}
          <form
            className="inspection-save"
            onSubmit={(e) => {
              e.preventDefault();
              void save();
            }}
          >
            <div>
              <span className="eyebrow">03 / KEEP THE GOOD VERSION</span>
              <h2>把這個想法，留在語法庫。</h2>
              <p>另存私人副本，確認後再從詳情頁分享給團隊。</p>
            </div>
            <div className="inspection-save-fields">
              <label htmlFor="inspection-name">新語法名稱</label>
              <input
                id="inspection-name"
                className="input"
                maxLength={100}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="例如：飲料口碑 · 去除廣告"
              />
              <button
                type="submit"
                className="btn btn-primary"
                disabled={
                  busy ||
                  loading ||
                  !check.result?.valid ||
                  check.pending ||
                  !name.trim() ||
                  Array.from(expression).length > 5000
                }
              >
                {busy ? <Spinner /> : <Save className="size-4" />} 另存私人語法
              </button>
            </div>
            {Array.from(expression).length > 5000 && (
              <p className="hint">
                草稿可檢視較長的組合，但一般語法最多 5,000
                字元；請縮短後再儲存。
              </p>
            )}
            {saved && (
              <p role="status" className="inspection-success">
                私人語法已儲存。
                <Link to={`/keywords/${saved}`}>開啟新語法 →</Link>
              </p>
            )}
          </form>
        </div>
      </div>
    </PageBody>
  );
}
