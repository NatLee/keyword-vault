import { ChevronLeft, ChevronRight, Download, FileUp, KeyRound, MessageSquare, Plus, Search, X } from "lucide-react";
import { useEffect, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";
import { api } from "../api";
import { Expression } from "../components/Expression";
import { ImportDialog } from "../components/ImportDialog";
import { FavoriteButton, KeywordBadges, TagLink } from "../components/KeywordBits";
import { KeywordEditor } from "../components/KeywordEditor";
import { PageBody } from "../components/Layout";
import { Empty, ErrorBanner, Loading, PageHeader, useToast } from "../components/ui";
import { timeAgo } from "../lib/format";
import { errorText, useAsync, useDebounced } from "../lib/hooks";
import type { Keyword, Kind, ListFilter, Scope, Sort } from "../types";

const PAGE_SIZE = 20;

const SCOPES: { value: Scope; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "mine", label: "我的" },
  { value: "shared", label: "別人分享的" },
  { value: "favorites", label: "收藏" },
];

function oneOf<T extends string>(value: string | null, allowed: readonly T[], fallback: T): T {
  return allowed.includes(value as T) ? (value as T) : fallback;
}

/** 篩選條件放在網址裡，重新整理或把網址貼給別人都會看到同一份清單。 */
function useFilter() {
  const [params, setParams] = useSearchParams();
  const filter = {
    q: params.get("q") ?? "",
    tag: params.get("tag") ?? "",
    scope: oneOf<Scope>(params.get("scope"), ["all", "mine", "shared", "favorites"], "all"),
    kind: oneOf<Kind | "">(params.get("kind"), ["plain", "combination"], ""),
    sort: oneOf<Sort>(params.get("sort"), ["updated", "created", "theme"], "updated"),
    page: Math.max(1, Number(params.get("page")) || 1),
  };
  const update = (changes: Partial<typeof filter>) => {
    const next = { ...filter, page: 1, ...changes };
    const out = new URLSearchParams();
    if (next.q) out.set("q", next.q);
    if (next.tag) out.set("tag", next.tag);
    if (next.scope !== "all") out.set("scope", next.scope);
    if (next.kind) out.set("kind", next.kind);
    if (next.sort !== "updated") out.set("sort", next.sort);
    if (next.page > 1) out.set("page", String(next.page));
    setParams(out, { replace: true });
  };
  return [filter, update] as const;
}

function KeywordRow({ keyword, onFavorite }: { keyword: Keyword; onFavorite: () => void }) {
  return (
    <li className="keyword-record flex gap-3 px-4 py-4 sm:px-5">
      <span className="record-index" aria-hidden="true">{String(keyword.id).padStart(3, "0")}</span>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <Link to={`/keywords/${keyword.id}`} className="font-medium hover:text-brand-700 dark:hover:text-brand-300">
            {keyword.theme}
          </Link>
          <KeywordBadges keyword={keyword} />
          {keyword.tags.map((t) => (
            <TagLink key={t} tag={t} />
          ))}
        </div>
        <Link to={`/keywords/${keyword.id}`} className="record-expression mt-2 block text-zinc-700 dark:text-zinc-200" tabIndex={-1}>
          <Expression value={keyword.expression} clamp />
        </Link>
        <p className="mt-1.5 flex flex-wrap items-center gap-x-3 text-xs text-zinc-500 dark:text-zinc-400">
          <span className="font-mono">#{keyword.id}</span>
          <span>{keyword.owner.display_name}</span>
          <span>{timeAgo(keyword.updated_at)}更新</span>
          {keyword.comment_count > 0 && (
            <span className="inline-flex items-center gap-1">
              <MessageSquare className="size-3" /> {keyword.comment_count}
            </span>
          )}
        </p>
      </div>
      <div className="shrink-0">
        <FavoriteButton favorite={keyword.favorite} onToggle={onFavorite} />
      </div>
    </li>
  );
}

export function Keywords() {
  const toast = useToast();
  const navigate = useNavigate();
  const [filter, update] = useFilter();
  const [search, setSearch] = useState(filter.q);
  const debounced = useDebounced(search, 300);
  const [creating, setCreating] = useState(false);
  const [importing, setImporting] = useState(false);

  // 搜尋框停下來之後才寫進網址
  useEffect(() => {
    if (debounced !== filter.q) update({ q: debounced });
    // 只在輸入變了的時候同步
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [debounced]);

  const query: ListFilter = {
    q: filter.q,
    tag: filter.tag,
    scope: filter.scope,
    kind: filter.kind || undefined,
    sort: filter.sort,
  };
  const { data, setData, error, loading, reload } = useAsync(
    () => api.keywords({ ...query, page: filter.page, page_size: PAGE_SIZE }),
    [filter.q, filter.tag, filter.scope, filter.kind, filter.sort, filter.page],
  );
  const tags = useAsync(() => api.tags(), []);

  const toggleFavorite = async (keyword: Keyword) => {
    try {
      const { favorite } = await api.setFavorite(keyword.id, !keyword.favorite);
      setData((page) => page && { ...page, items: page.items.map((k) => (k.id === keyword.id ? { ...k, favorite } : k)) });
    } catch (e) {
      toast(errorText(e), "error");
    }
  };

  const pages = data ? Math.max(1, Math.ceil(data.total / data.page_size)) : 1;
  const filtered = Boolean(filter.q || filter.tag || filter.kind || filter.scope !== "all");

  return (
    <PageBody>
      <PageHeader
        title="語法庫"
        subtitle="收藏值得重用的搜尋語法。搜尋主題、挑選標籤，或組合下一個查詢。"
        actions={
          <>
            <button className="btn btn-secondary" onClick={() => setImporting(true)}>
              <FileUp className="size-4" /> 匯入
            </button>
            <a className="btn btn-secondary" href={api.exportUrl(query)} download>
              <Download className="size-4" /> 匯出 CSV
            </a>
            <button className="btn btn-primary" onClick={() => setCreating(true)}>
              <Plus className="size-4" /> 新增關鍵字
            </button>
          </>
        }
      />

      <div className="library-filters card mb-5 flex flex-col gap-4 p-3 sm:p-4">
        <div className="flex flex-wrap items-center gap-2">
          <label className="relative min-w-52 flex-1">
            <span className="sr-only">搜尋</span>
            <Search className="pointer-events-none absolute top-2.5 left-3 size-4 text-zinc-400" />
            <input
              className="input pl-9"
              placeholder="搜尋主題或語法裡的字…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </label>
          <select
            className="input w-auto"
            aria-label="種類"
            value={filter.kind}
            onChange={(e) => update({ kind: e.target.value as Kind | "" })}
          >
            <option value="">所有種類</option>
            <option value="plain">一般關鍵字</option>
            <option value="combination">組合關鍵字</option>
          </select>
          <select
            className="input w-auto"
            aria-label="排序"
            value={filter.sort}
            onChange={(e) => update({ sort: e.target.value as Sort })}
          >
            <option value="updated">最近更新</option>
            <option value="created">最近新增</option>
            <option value="theme">主題</option>
          </select>
        </div>
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
          <div className="flex rounded-lg bg-zinc-100 p-0.5 text-sm dark:bg-zinc-800" role="tablist" aria-label="範圍">
            {SCOPES.map((s) => (
              <button
                key={s.value}
                role="tab"
                aria-selected={filter.scope === s.value}
                onClick={() => update({ scope: s.value })}
                className={`rounded-md px-3 py-1 transition-colors ${
                  filter.scope === s.value
                    ? "bg-white font-medium text-brand-700 shadow-sm dark:bg-zinc-950 dark:text-brand-300"
                    : "text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-100"
                }`}
              >
                {s.label}
              </button>
            ))}
          </div>
          {tags.data && tags.data.length > 0 && (
            <div className="flex flex-wrap items-center gap-1.5">
              {tags.data.slice(0, 12).map((t) => {
                const active = filter.tag === t.name;
                return (
                  <button
                    key={t.name}
                    onClick={() => update({ tag: active ? "" : t.name })}
                    aria-pressed={active}
                    className={`rounded-md px-1.5 py-0.5 text-xs transition-colors ${
                      active
                        ? "bg-brand-600 text-white dark:bg-brand-500 dark:text-zinc-950"
                        : "bg-zinc-100 text-zinc-600 hover:bg-zinc-200 dark:bg-zinc-800 dark:text-zinc-300 dark:hover:bg-zinc-700"
                    }`}
                  >
                    #{t.name} <span className="opacity-60">{t.count}</span>
                  </button>
                );
              })}
              {filter.tag && !tags.data.slice(0, 12).some((t) => t.name === filter.tag) && (
                <button
                  onClick={() => update({ tag: "" })}
                  className="inline-flex items-center gap-1 rounded-md bg-brand-600 px-1.5 py-0.5 text-xs text-white dark:bg-brand-500 dark:text-zinc-950"
                >
                  #{filter.tag} <X className="size-3" />
                </button>
              )}
            </div>
          )}
        </div>
      </div>

      {error && <ErrorBanner message={error} />}
      {loading && !data && <Loading />}
      {data && (
        <section className={`card ${loading ? "opacity-70" : ""}`}>
          <div className="library-caption"><span>EXPRESSION INDEX</span><span>{data.total.toLocaleString()} 個語法 / {filter.sort === "theme" ? "依主題" : filter.sort === "created" ? "最近新增" : "最近更新"}</span></div>
          {data.items.length === 0 ? (
            <Empty icon={KeyRound} title={filtered ? "沒有符合條件的關鍵字" : "還沒有關鍵字"}>
              {filtered ? "換個搜尋的字，或把篩選條件拿掉。" : "按右上角的「新增關鍵字」寫第一個，或從 CSV 匯入。"}
            </Empty>
          ) : (
            <ul className="divide-y divide-zinc-100 dark:divide-zinc-800">
              {data.items.map((k) => (
                <KeywordRow key={k.id} keyword={k} onFavorite={() => void toggleFavorite(k)} />
              ))}
            </ul>
          )}
          <div className="flex items-center justify-between border-t border-zinc-100 px-4 py-2.5 text-sm text-zinc-500 sm:px-5 dark:border-zinc-800 dark:text-zinc-400">
            <span>共 {data.total.toLocaleString()} 個</span>
            {pages > 1 && (
              <span className="flex items-center gap-1">
                <button
                  className="btn btn-ghost px-1.5"
                  disabled={filter.page <= 1}
                  onClick={() => update({ page: filter.page - 1 })}
                  aria-label="上一頁"
                >
                  <ChevronLeft className="size-4" />
                </button>
                <span className="tabular-nums">
                  {filter.page} / {pages}
                </span>
                <button
                  className="btn btn-ghost px-1.5"
                  disabled={filter.page >= pages}
                  onClick={() => update({ page: filter.page + 1 })}
                  aria-label="下一頁"
                >
                  <ChevronRight className="size-4" />
                </button>
              </span>
            )}
          </div>
        </section>
      )}

      <KeywordEditor
        open={creating}
        onClose={() => setCreating(false)}
        onSaved={(k) => {
          setCreating(false);
          navigate(`/keywords/${k.id}`);
        }}
      />
      <ImportDialog
        open={importing}
        onClose={() => setImporting(false)}
        onImported={() => {
          setImporting(false);
          void reload();
          void tags.reload();
        }}
      />
    </PageBody>
  );
}
