import { Plus, Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";
import { parseTags, VISIBILITY_HINT, VISIBILITY_LABEL } from "../lib/format";
import { errorText, useDebounced } from "../lib/hooks";
import type { Keyword, KeywordInput, Visibility } from "../types";
import { Expression } from "./Expression";
import { CheckResultView, useCheck } from "./SyntaxCheck";
import { Dialog, ErrorBanner, Spinner, useToast } from "./ui";
import { QueryAnatomy } from "./QueryAnatomy";

const VISIBILITIES: Visibility[] = ["private", "group", "public"];

/** 組合方式裡用到的編號，照出現順序。 */
export function logicIds(logic: string): number[] {
  const ids: number[] = [];
  for (const m of logic.matchAll(/[#＃]?\s*(\d+)/g)) {
    const id = Number(m[1]);
    if (!ids.includes(id)) ids.push(id);
  }
  return ids;
}

/** 把組合方式裡的編號換成 `(語法)`；有不認得的編號就回 null。 */
export function expandLogic(logic: string, expressions: Map<number, string>): string | null {
  let missing = false;
  const expanded = logic.replace(/[#＃]?\s*(\d+)/g, (_, digits: string) => {
    const expression = expressions.get(Number(digits));
    if (expression === undefined) missing = true;
    return `(${expression ?? ""})`;
  });
  return missing ? null : expanded;
}

/** 組合關鍵字：找關鍵字、點一下把編號放進組合方式。 */
function LogicPicker({
  logic,
  onChange,
  known,
  remember,
  excludeId,
}: {
  logic: string;
  onChange: (logic: string) => void;
  known: Map<number, Keyword>;
  remember: (keywords: Keyword[]) => void;
  excludeId?: number;
}) {
  const [q, setQ] = useState("");
  const search = useDebounced(q, 250);
  const [results, setResults] = useState<Keyword[] | null>(null);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    let stale = false;
    api
      .keywords({ q: search, page_size: 8, sort: "updated" })
      .then((page) => {
        if (stale) return;
        setResults(page.items);
        remember(page.items);
      })
      .catch(() => !stale && setResults([]));
    return () => {
      stale = true;
    };
  }, [search, remember]);

  const insert = (text: string) => {
    onChange(logic + text);
    input.current?.focus();
  };
  const used = logicIds(logic);

  return (
    <div>
      <label className="label" htmlFor="kw-logic">
        組合方式
      </label>
      <input
        id="kw-logic"
        ref={input}
        className="input font-mono"
        placeholder="(#3|#5)!#8"
        value={logic}
        onChange={(e) => onChange(e.target.value)}
      />
      <div className="mt-2 flex flex-wrap items-center gap-2">
        {["&", "|", "!", "(", ")"].map((op) => (
          <button key={op} type="button" className="btn btn-secondary px-3 py-1 font-mono" onClick={() => insert(op)}>
            {op}
          </button>
        ))}
        <span className="hint mt-0 ml-1">用編號代表關鍵字，存檔時每個編號會換成（它的語法）。</span>
      </div>

      {used.length > 0 && (
        <ul className="mt-3 flex flex-col gap-1 text-sm">
          {used.map((id) => (
            <li key={id} className="flex items-baseline gap-2">
              <span className="rounded bg-sky-100 px-1 font-mono text-xs text-sky-800 dark:bg-sky-950 dark:text-sky-300">
                #{id}
              </span>
              <span className={known.has(id) ? "" : "text-zinc-400"}>{known.get(id)?.theme ?? "（還沒載入或看不到）"}</span>
            </li>
          ))}
        </ul>
      )}

      <div className="mt-3 rounded border border-zinc-200 dark:border-zinc-800">
        <label className="relative block border-b border-zinc-200 dark:border-zinc-800">
          <span className="visually-hidden">找關鍵字</span>
          <Search className="pointer-events-none absolute top-2.5 left-3 size-4 text-zinc-400" />
          <input
            className="w-full bg-transparent py-2 pr-3 pl-9 text-sm focus:outline-none"
            placeholder="找要拿來組合的關鍵字…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </label>
        <ul className="max-h-44 overflow-y-auto">
          {results === null && (
            <li className="flex items-center gap-2 px-3 py-2 text-sm text-zinc-500">
              <Spinner /> 載入中…
            </li>
          )}
          {results?.length === 0 && <li className="px-3 py-2 text-sm text-zinc-500">找不到關鍵字</li>}
          {results
            ?.filter((k) => k.id !== excludeId)
            .map((k) => (
              <li key={k.id}>
                <button
                  type="button"
                  className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-zinc-50 dark:hover:bg-zinc-800/60"
                  onClick={() => insert(`#${k.id}`)}
                  title={`把 #${k.id} 放進組合方式`}
                >
                  <span className="w-10 shrink-0 font-mono text-xs text-zinc-400">#{k.id}</span>
                  <span className="min-w-0 flex-1 truncate">{k.theme}</span>
                  <span className="hidden max-w-[45%] truncate font-mono text-xs text-zinc-400 sm:block">{k.expression}</span>
                  <Plus className="size-4 shrink-0 text-brand-600 dark:text-brand-300" />
                </button>
              </li>
            ))}
        </ul>
      </div>
    </div>
  );
}

/**
 * 新增或編輯關鍵字。`keyword` 有值是編輯：一般關鍵字改語法，組合關鍵字改組合方式，
 * 兩種不能互換。
 */
export function KeywordEditor({
  open,
  keyword,
  onClose,
  onSaved,
}: {
  open: boolean;
  keyword?: Keyword;
  onClose: () => void;
  onSaved: (keyword: Keyword) => void;
}) {
  const toast = useToast();
  const [theme, setTheme] = useState("");
  const [combination, setCombination] = useState(false);
  const [expression, setExpression] = useState("");
  const [logic, setLogic] = useState("");
  const [visibility, setVisibility] = useState<Visibility>("private");
  const [tags, setTags] = useState("");
  const [known, setKnown] = useState<Map<number, Keyword>>(new Map());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setTheme(keyword?.theme ?? "");
    setCombination(keyword?.is_combination ?? false);
    setExpression(keyword && !keyword.is_combination ? keyword.expression : "");
    setLogic(keyword?.logic ?? "");
    setVisibility(keyword?.visibility ?? "private");
    setTags(keyword?.tags.join(", ") ?? "");
    setError(null);
  }, [open, keyword]);

  const remember = useMemo(
    () => (keywords: Keyword[]) =>
      setKnown((prev) => {
        const next = new Map(prev);
        for (const k of keywords) next.set(k.id, k);
        return next;
      }),
    [],
  );

  // 組合方式裡還不認得的編號，去問一次
  useEffect(() => {
    if (!open || !combination) return;
    const unknown = logicIds(logic).filter((id) => !known.has(id));
    if (unknown.length === 0) return;
    const timer = window.setTimeout(() => {
      void Promise.all(unknown.map((id) => api.keyword(id).catch(() => null))).then((found) =>
        remember(found.filter((k): k is NonNullable<typeof k> => k !== null)),
      );
    }, 400);
    return () => window.clearTimeout(timer);
  }, [open, combination, logic, known, remember]);

  const expanded = useMemo(() => {
    if (!combination || !logic.trim()) return null;
    return expandLogic(logic, new Map(Array.from(known, ([id, k]) => [id, k.expression])));
  }, [combination, logic, known]);

  const check = useCheck(combination ? (expanded ?? "") : expression);
  const body = combination ? logic.trim() : expression.trim();
  const canSave = theme.trim() !== "" && body !== "" && check.result?.valid === true && !check.pending && !busy;

  const save = async () => {
    setBusy(true);
    setError(null);
    const input: KeywordInput = { theme: theme.trim(), visibility, tags: parseTags(tags) };
    if (combination) input.logic = logic.trim();
    else input.expression = expression.trim();
    try {
      if (keyword) {
        const { keyword: saved, cascaded } = await api.updateKeyword(keyword.id, input);
        toast(cascaded > 0 ? `已儲存，另外更新了 ${cascaded} 個用到它的組合關鍵字` : "已儲存");
        onSaved(saved);
      } else {
        const saved = await api.createKeyword(input);
        toast(`已新增「${saved.theme}」`);
        onSaved(saved);
      }
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      wide
      title={keyword ? "編輯關鍵字" : "新增關鍵字"}
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            取消
          </button>
          <button type="button" className="btn btn-primary" disabled={!canSave} onClick={() => void save()}>
            {busy && <Spinner />}
            {keyword ? "儲存" : "新增"}
          </button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <div>
          <label className="label" htmlFor="kw-theme">
            主題
          </label>
          <input
            id="kw-theme"
            className="input"
            placeholder="例如：手搖飲、電動車"
            value={theme}
            onChange={(e) => setTheme(e.target.value)}
          />
        </div>

        {!keyword && (
          <div className="flex rounded bg-zinc-100 p-1 text-sm dark:bg-zinc-800" role="radiogroup" aria-label="種類">
            {[
              { value: false, label: "一般關鍵字", hint: "自己寫語法" },
              { value: true, label: "組合關鍵字", hint: "用現有的關鍵字組起來" },
            ].map((o) => (
              <button
                key={o.label}
                type="button"
                role="radio"
                aria-checked={combination === o.value}
                onClick={() => setCombination(o.value)}
                className={`flex-1 rounded px-3 py-2 transition-colors ${
                  combination === o.value
                    ? "bg-white font-semibold text-brand-700 shadow-none dark:bg-zinc-950 dark:text-brand-300"
                    : "text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-100"
                }`}
              >
                {o.label}
                <span className="ml-2 hidden text-xs font-normal text-zinc-400 sm:inline">{o.hint}</span>
              </button>
            ))}
          </div>
        )}

        {combination ? (
          <LogicPicker logic={logic} onChange={setLogic} known={known} remember={remember} excludeId={keyword?.id} />
        ) : (
          <div>
            <label className="label" htmlFor="kw-expression">
              關鍵字語法
            </label>
            <textarea
              id="kw-expression"
              className="input expr min-h-28"
              placeholder="(珍珠奶茶|(珍珠 鮮奶 ~))&手搖!廣告"
              spellCheck={false}
              value={expression}
              onChange={(e) => setExpression(e.target.value)}
            />
            <p className="hint">
              <code>&amp;</code> 而且、<code>|</code> 或、<code>!</code> 排除、<code>(a b ~5)</code> 片語 slop 設為 5（非字數）。
            </p>
          </div>
        )}

        {combination && expanded && (
          <div>
            <p className="label">展開之後</p>
            <div className="rounded bg-zinc-100 px-3 py-2 dark:bg-zinc-950">
              <Expression value={expanded} />
            </div>
          </div>
        )}
        <CheckResultView {...check} />
        {check.result?.valid && <details className="editor-anatomy"><summary>拆開查看語法結構</summary><QueryAnatomy clauses={check.result.clauses} compact /></details>}

        <div>
          <p className="label">分享範圍</p>
          <div className="grid grid-cols-3 gap-2">
            {VISIBILITIES.map((v) => (
              <label
                key={v}
                className={`cursor-pointer rounded border px-3 py-2 text-sm transition-colors ${
                  visibility === v
                    ? "border-brand-500 bg-brand-50 dark:border-brand-400 dark:bg-brand-950/50"
                    : "border-zinc-200 hover:border-zinc-300 dark:border-zinc-800 dark:hover:border-zinc-700"
                }`}
              >
                <input
                  type="radio"
                  name="kw-visibility"
                  className="visually-hidden"
                  checked={visibility === v}
                  onChange={() => setVisibility(v)}
                />
                <span className="block font-semibold">{VISIBILITY_LABEL[v]}</span>
                <span className="block text-xs text-zinc-500 dark:text-zinc-400">{VISIBILITY_HINT[v]}</span>
              </label>
            ))}
          </div>
        </div>

        <div>
          <label className="label" htmlFor="kw-tags">
            標籤
          </label>
          <input
            id="kw-tags"
            className="input"
            placeholder="飲料, 熱門"
            value={tags}
            onChange={(e) => setTags(e.target.value)}
          />
          <p className="hint">用逗號或空白分開，最多 10 個。</p>
        </div>

        {error && <ErrorBanner message={error} />}
      </div>
    </Dialog>
  );
}
