import {
  ArrowLeft,
  Combine,
  History,
  MessageSquare,
  Pencil,
  Send,
  Trash2,
} from "lucide-react";
import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { api } from "../api";
import { CopyButton, DiffView, Expression } from "../components/Expression";
import {
  FavoriteButton,
  KeywordBadges,
  TagLink,
} from "../components/KeywordBits";
import { KeywordEditor } from "../components/KeywordEditor";
import { QueryAnatomy } from "../components/QueryAnatomy";
import { PageBody } from "../components/Layout";
import {
  Dialog,
  ErrorBanner,
  Loading,
  PageHeader,
  Spinner,
  useToast,
} from "../components/ui";
import { dateTime, timeAgo } from "../lib/format";
import { errorText, useAsync } from "../lib/hooks";
import type { Comment, HistoryEntry, KeywordDetail, Linked } from "../types";

function LinkedList({ items }: { items: Linked[] }) {
  return (
    <ul className="flex flex-col gap-2 text-sm">
      {items.map((k) => (
        <li key={k.id} className="flex items-center gap-2">
          <span className="rounded bg-sky-100 px-1 font-mono text-xs text-sky-800 dark:bg-sky-950 dark:text-sky-300">
            #{k.id}
          </span>
          {k.theme === null ? (
            <span className="text-zinc-400">（你看不到這個關鍵字）</span>
          ) : (
            <Link
              to={`/keywords/${k.id}`}
              className="hover:text-brand-700 hover:underline dark:hover:text-brand-300"
            >
              {k.theme}
            </Link>
          )}
        </li>
      ))}
    </ul>
  );
}

/** 一筆修改紀錄：展開後比對改之前和改之後。 */
function HistoryItem({ entry, after }: { entry: HistoryEntry; after: string }) {
  const [open, setOpen] = useState(false);
  const diff = useAsync(
    () =>
      open ? api.diff(entry.expression, after) : Promise.resolve(undefined),
    [open, entry.id, after],
  );
  return (
    <li className="px-4 py-3 sm:px-5">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-3 text-left text-sm"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <span>
          <span className="font-semibold">
            {entry.user_name ?? "已移除的帳號"}
          </span>
          <span className="text-zinc-500 dark:text-zinc-400"> 改了語法</span>
        </span>
        <span
          className="shrink-0 text-xs text-zinc-500 dark:text-zinc-400"
          title={dateTime(entry.created_at)}
        >
          {timeAgo(entry.created_at)}
        </span>
      </button>
      {open && (
        <div className="mt-2 rounded bg-zinc-50 px-3 py-2 dark:bg-zinc-950">
          {diff.error ? (
            <ErrorBanner message={diff.error} />
          ) : diff.data ? (
            <DiffView segments={diff.data.segments} />
          ) : (
            <Spinner />
          )}
        </div>
      )}
    </li>
  );
}

function Comments({ keywordId }: { keywordId: number }) {
  const toast = useToast();
  const { data, error, reload } = useAsync(
    () => api.comments(keywordId),
    [keywordId],
  );
  const [body, setBody] = useState("");
  const [busy, setBusy] = useState(false);
  const [editing, setEditing] = useState<{ id: number; body: string } | null>(
    null,
  );

  const run = async (action: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await action();
      await reload();
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      setBusy(false);
    }
  };

  const add = () =>
    run(async () => {
      await api.addComment(keywordId, body.trim());
      setBody("");
    });
  const save = (comment: Comment) =>
    run(async () => {
      if (editing) await api.updateComment(comment.id, editing.body.trim());
      setEditing(null);
    });

  return (
    <section className="card">
      <h2 className="flex items-center gap-2 border-b border-zinc-100 px-4 py-3 text-sm font-semibold sm:px-5 dark:border-zinc-800">
        <MessageSquare className="size-4 text-zinc-400" /> 留言
        {data ? `（${data.length}）` : ""}
      </h2>
      {error && (
        <div className="p-4">
          <ErrorBanner message={error} />
        </div>
      )}
      {data && data.length > 0 && (
        <ul className="divide-y divide-zinc-100 dark:divide-zinc-800">
          {data.map((c) => (
            <li key={c.id} className="px-4 py-3 text-sm sm:px-5">
              <div className="flex items-center justify-between gap-2">
                <p>
                  <span className="font-semibold">{c.user_name}</span>
                  <span
                    className="ml-2 text-xs text-zinc-500 dark:text-zinc-400"
                    title={dateTime(c.created_at)}
                  >
                    {timeAgo(c.created_at)}
                    {c.updated_at !== c.created_at && "（編輯過）"}
                  </span>
                </p>
                <span className="flex shrink-0">
                  {c.can_edit && (
                    <button
                      className="btn btn-ghost px-2 py-1"
                      title="編輯留言"
                      onClick={() => setEditing({ id: c.id, body: c.body })}
                    >
                      <Pencil className="size-4" />
                    </button>
                  )}
                  {c.can_delete && (
                    <button
                      className="btn btn-ghost px-2 py-1"
                      title="刪除留言"
                      disabled={busy}
                      onClick={() => {
                        if (window.confirm("要刪掉這則留言嗎？"))
                          void run(() => api.deleteComment(c.id));
                      }}
                    >
                      <Trash2 className="size-4" />
                    </button>
                  )}
                </span>
              </div>
              {editing?.id === c.id ? (
                <div className="mt-2 flex flex-col gap-2">
                  <textarea
                    className="input min-h-20"
                    aria-label="編輯留言"
                    value={editing.body}
                    onChange={(e) =>
                      setEditing({ id: c.id, body: e.target.value })
                    }
                  />
                  <div className="flex justify-end gap-2">
                    <button
                      className="btn btn-secondary"
                      onClick={() => setEditing(null)}
                    >
                      取消
                    </button>
                    <button
                      className="btn btn-primary"
                      disabled={busy || !editing.body.trim()}
                      onClick={() => void save(c)}
                    >
                      儲存
                    </button>
                  </div>
                </div>
              ) : (
                <p className="mt-1 whitespace-pre-wrap break-words text-zinc-700 dark:text-zinc-300">
                  {c.body}
                </p>
              )}
            </li>
          ))}
        </ul>
      )}
      <form
        className="flex flex-col gap-2 border-t border-zinc-100 p-4 sm:px-5 dark:border-zinc-800"
        onSubmit={(e) => {
          e.preventDefault();
          void add();
        }}
      >
        <textarea
          className="input min-h-20"
          aria-label="留言"
          placeholder="這個關鍵字用在哪裡、有什麼要注意的…"
          value={body}
          onChange={(e) => setBody(e.target.value)}
        />
        <button
          className="btn btn-primary self-end"
          type="submit"
          disabled={busy || !body.trim()}
        >
          <Send className="size-4" /> 留言
        </button>
      </form>
    </section>
  );
}

function DeleteDialog({
  keyword,
  open,
  onClose,
  onDeleted,
}: {
  keyword: KeywordDetail;
  open: boolean;
  onClose: () => void;
  onDeleted: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cascade = keyword.dependents.length > 0;

  const remove = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.deleteKeyword(keyword.id, cascade);
      onDeleted();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      title="刪除關鍵字"
      onClose={onClose}
      footer={
        <>
          <button className="btn btn-secondary" onClick={onClose}>
            取消
          </button>
          <button
            className="btn btn-danger"
            disabled={busy}
            onClick={() => void remove()}
          >
            {busy ? <Spinner /> : <Trash2 className="size-4" />}
            {cascade
              ? `連同 ${keyword.dependents.length} 個組合關鍵字一起刪除`
              : "刪除"}
          </button>
        </>
      }
    >
      <div className="flex flex-col gap-3 text-sm">
        <p>
          要刪掉「<span className="font-semibold">{keyword.theme}</span>
          」嗎？修改紀錄和留言也會一起刪掉，刪了就拿不回來。
        </p>
        {cascade && (
          <div className="rounded border border-amber-200 bg-amber-50 p-3 dark:border-amber-900 dark:bg-amber-950/40">
            <p className="mb-2 text-amber-900 dark:text-amber-200">
              下面這些組合關鍵字用到它，會一起被刪掉：
            </p>
            <LinkedList items={keyword.dependents} />
          </div>
        )}
        {error && <ErrorBanner message={error} />}
      </div>
    </Dialog>
  );
}

export function KeywordDetailPage() {
  const id = Number(useParams().id);
  const navigate = useNavigate();
  const toast = useToast();
  const {
    data: keyword,
    setData,
    error,
    reload,
  } = useAsync(() => api.keyword(id), [id]);
  const history = useAsync(() => api.history(id), [id]);
  const [editing, setEditing] = useState(false);
  const [deleting, setDeleting] = useState(false);

  if (error) {
    return (
      <PageBody>
        <ErrorBanner message={error} />
        <Link to="/keywords" className="btn btn-secondary mt-4">
          <ArrowLeft className="size-4" /> 回到關鍵字
        </Link>
      </PageBody>
    );
  }
  if (!keyword) {
    return (
      <PageBody>
        <Loading />
      </PageBody>
    );
  }

  const toggleFavorite = async () => {
    try {
      const { favorite } = await api.setFavorite(keyword.id, !keyword.favorite);
      setData((k) => k && { ...k, favorite });
    } catch (e) {
      toast(errorText(e), "error");
    }
  };

  return (
    <PageBody>
      <Link
        to="/keywords"
        className="mb-3 inline-flex items-center gap-1 text-sm text-zinc-500 hover:text-brand-700 dark:text-zinc-400 dark:hover:text-brand-300"
      >
        <ArrowLeft className="size-4" /> 關鍵字
      </Link>
      <PageHeader
        title={
          <span className="flex flex-wrap items-center gap-2">
            {keyword.theme}
            <KeywordBadges keyword={keyword} />
          </span>
        }
        subtitle={
          <>
            <span className="font-mono">#{keyword.id}</span>・
            {keyword.owner.display_name}・{dateTime(keyword.created_at)} 建立・
            {timeAgo(keyword.updated_at)}更新
          </>
        }
        actions={
          <>
            <Link
              className="btn btn-secondary"
              to={`/inspect?keyword=${keyword.id}`}
            >
              <Combine className="size-4" /> 拆解與試寫
            </Link>
            <FavoriteButton
              favorite={keyword.favorite}
              onToggle={() => void toggleFavorite()}
              withLabel
            />
            {keyword.can_edit && (
              <button
                className="btn btn-secondary"
                onClick={() => setEditing(true)}
              >
                <Pencil className="size-4" /> 編輯
              </button>
            )}
            {keyword.can_delete && (
              <button
                className="btn btn-danger"
                onClick={() => setDeleting(true)}
              >
                <Trash2 className="size-4" /> 刪除
              </button>
            )}
          </>
        }
      />

      <div className="detail-archive-label">
        <span>ARCHIVE RECORD / {String(keyword.id).padStart(3, "0")}</span>
        <span>
          {keyword.is_combination
            ? "組合索引 · 連結館藏"
            : "一般手稿 · 獨立語法"}
        </span>
        <span>01 語法結構 / 02 編譯輸出 / 03 修訂記錄</span>
      </div>
      <div className="mb-6">
        <QueryAnatomy
          key={`${keyword.id}:${keyword.expression}`}
          clauses={keyword.clauses}
        />
      </div>
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <div className="flex flex-col gap-4 lg:col-span-2">
          <section className="card p-4 sm:p-5">
            <div className="mb-2 flex items-center justify-between">
              <h2 className="text-sm font-semibold">關鍵字語法</h2>
              <span className="flex">
                <Link
                  className="btn btn-ghost px-2 py-1 text-xs"
                  to={`/compare?a=${encodeURIComponent(keyword.expression)}`}
                >
                  拿去比對
                </Link>
                <CopyButton text={keyword.expression} />
              </span>
            </div>
            <div className="rounded bg-zinc-50 px-3 py-3 dark:bg-zinc-950">
              <Expression value={keyword.expression} />
            </div>

            {keyword.logic && (
              <div className="mt-5">
                <h2 className="mb-2 flex items-center gap-2 text-sm font-semibold">
                  <Combine className="size-4 text-zinc-400" /> 組合方式
                </h2>
                <div className="mb-3 rounded bg-zinc-50 px-3 py-3 dark:bg-zinc-950">
                  <Expression value={keyword.logic} />
                </div>
                <LinkedList items={keyword.refs} />
                <p className="hint">
                  被引用的關鍵字改了語法，這個組合關鍵字會跟著更新。
                </p>
              </div>
            )}

            <div className="mt-5">
              <div className="mb-2 flex items-center justify-between">
                <h2 className="text-sm font-semibold">
                  送給搜尋引擎的 query_string
                </h2>
                <CopyButton text={keyword.query_string} />
              </div>
              <code className="expr block rounded bg-zinc-50 px-3 py-3 text-zinc-700 dark:bg-zinc-950 dark:text-zinc-300">
                {keyword.query_string}
              </code>
            </div>
          </section>

          <Comments keywordId={keyword.id} />
        </div>

        <div className="flex flex-col gap-4">
          <section className="card p-4 sm:p-5">
            <h2 className="mb-2 text-sm font-semibold">標籤</h2>
            {keyword.tags.length > 0 ? (
              <div className="flex flex-wrap gap-2">
                {keyword.tags.map((t) => (
                  <TagLink key={t} tag={t} />
                ))}
              </div>
            ) : (
              <p className="text-sm text-zinc-500 dark:text-zinc-400">
                沒有標籤
              </p>
            )}
            <h2 className="mt-5 mb-2 text-sm font-semibold">
              用到的詞（{keyword.terms.length}）
            </h2>
            <div className="flex flex-wrap gap-2">
              {keyword.terms.map((t, i) => (
                <span
                  key={i}
                  className="rounded border border-zinc-200 px-2 py-1 font-mono text-xs dark:border-zinc-700"
                >
                  {t}
                </span>
              ))}
            </div>
            {keyword.dependents.length > 0 && (
              <>
                <h2 className="mt-5 mb-2 text-sm font-semibold">
                  被這些組合關鍵字用到
                </h2>
                <LinkedList items={keyword.dependents} />
              </>
            )}
          </section>

          <section className="card">
            <h2 className="flex items-center gap-2 border-b border-zinc-100 px-4 py-3 text-sm font-semibold sm:px-5 dark:border-zinc-800">
              <History className="size-4 text-zinc-400" /> 修改紀錄
            </h2>
            {history.error && (
              <div className="p-4">
                <ErrorBanner message={history.error} />
              </div>
            )}
            {history.data && history.data.length === 0 && (
              <p className="px-4 py-4 text-sm text-zinc-500 sm:px-5 dark:text-zinc-400">
                語法還沒被改過。
              </p>
            )}
            {history.data && history.data.length > 0 && (
              <ul className="divide-y divide-zinc-100 dark:divide-zinc-800">
                {history.data.map((entry, i, all) => (
                  // 紀錄存的是改之前的語法，新的在前面：改之後就是上一筆，或現在的語法
                  <HistoryItem
                    key={entry.id}
                    entry={entry}
                    after={i === 0 ? keyword.expression : all[i - 1].expression}
                  />
                ))}
              </ul>
            )}
          </section>
        </div>
      </div>

      <KeywordEditor
        open={editing}
        keyword={keyword}
        onClose={() => setEditing(false)}
        onSaved={() => {
          setEditing(false);
          void reload();
          void history.reload();
        }}
      />
      <DeleteDialog
        keyword={keyword}
        open={deleting}
        onClose={() => setDeleting(false)}
        onDeleted={() => {
          toast("已刪除");
          navigate("/keywords");
        }}
      />
    </PageBody>
  );
}
