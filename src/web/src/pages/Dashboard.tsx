import { Combine, KeyRound, Library, Plus, Shuffle, Sparkles, Star, type LucideIcon } from "lucide-react";
import { useState } from "react";
import { Link, useNavigate } from "react-router";
import { api } from "../api";
import { CopyButton, Expression } from "../components/Expression";
import { KeywordBadges } from "../components/KeywordBits";
import { KeywordEditor } from "../components/KeywordEditor";
import { PageBody } from "../components/Layout";
import { Empty, ErrorBanner, Loading, PageHeader } from "../components/ui";
import { useUser } from "../context";
import { timeAgo } from "../lib/format";
import { useAsync } from "../lib/hooks";
import type { Keyword, Stats } from "../types";

function StatCard({
  icon: Icon,
  label,
  value,
  note,
  to,
}: {
  icon: LucideIcon;
  label: string;
  value: number;
  note?: string;
  to: string;
}) {
  return (
    <Link to={to} className="vault-stat flex items-center gap-3 p-4 transition-colors">
      <span className="grid size-10 shrink-0 place-items-center rounded-xl bg-brand-50 text-brand-600 dark:bg-brand-950 dark:text-brand-300">
        <Icon className="size-5" />
      </span>
      <span className="min-w-0">
        <span className="block text-xs text-zinc-500 dark:text-zinc-400">{label}</span>
        <span className="block text-2xl leading-tight font-semibold tabular-nums">{value.toLocaleString()}</span>
        {note && <span className="block truncate text-xs text-zinc-500 dark:text-zinc-400">{note}</span>}
      </span>
    </Link>
  );
}

/** 最近 12 個月每個月新增的關鍵字數量。 */
function MonthlyChart({ monthly }: { monthly: Stats["monthly"] }) {
  const max = Math.max(1, ...monthly.map((m) => m.count));
  return (
    <div className="flex h-44 items-end gap-1.5 sm:gap-2.5" role="img" aria-label="最近 12 個月新增的關鍵字數量">
      {monthly.map((m) => (
        <div key={m.month} className="flex h-full min-w-0 flex-1 flex-col items-center justify-end gap-1" title={`${m.month}：${m.count} 個`}>
          <span className="text-[11px] text-zinc-500 tabular-nums dark:text-zinc-400">{m.count || ""}</span>
          <div
            className="w-full rounded-t-md bg-brand-500/85 dark:bg-brand-400/80"
            style={{ height: `${(m.count / max) * 78}%`, minHeight: m.count ? 4 : 2 }}
          />
          <span className="text-[11px] text-zinc-400 tabular-nums">{Number(m.month.slice(5))}月</span>
        </div>
      ))}
    </div>
  );
}

function TodayCard({ initial }: { initial: Keyword | null }) {
  const [keyword, setKeyword] = useState(initial);
  const shuffle = async () => {
    const next = await api.randomKeyword().catch(() => null);
    if (next) setKeyword(next);
  };
  return (
    <section className="card flex flex-col p-5">
      <div className="mb-3 flex items-center justify-between gap-2">
        <h2 className="flex items-center gap-2 font-semibold">
          <Sparkles className="size-4 text-amber-500" /> 今日關鍵字
        </h2>
        {keyword && (
          <button className="btn btn-ghost px-2 py-1 text-xs" onClick={() => void shuffle()}>
            <Shuffle className="size-3.5" /> 換一個
          </button>
        )}
      </div>
      {keyword ? (
        <>
          <div className="mb-2 flex flex-wrap items-center gap-2">
            <Link to={`/keywords/${keyword.id}`} className="font-medium hover:text-brand-700 dark:hover:text-brand-300">
              {keyword.theme}
            </Link>
            <KeywordBadges keyword={keyword} />
          </div>
          <div className="flex-1 rounded-lg bg-zinc-100 px-3 py-2 dark:bg-zinc-950">
            <Expression value={keyword.expression} />
          </div>
          <div className="mt-2 flex items-center justify-between text-xs text-zinc-500 dark:text-zinc-400">
            <span>{keyword.owner.display_name} 分享</span>
            <CopyButton text={keyword.expression} label="複製語法" />
          </div>
        </>
      ) : (
        <p className="text-sm text-zinc-500 dark:text-zinc-400">還沒有人分享關鍵字。把你的關鍵字設成「群組」或「公開」，大家就看得到。</p>
      )}
    </section>
  );
}

export function Dashboard() {
  const user = useUser();
  const navigate = useNavigate();
  const { data, error, loading } = useAsync(() => api.stats(), []);
  const [creating, setCreating] = useState(false);

  return (
    <PageBody>
      <PageHeader
        title="語法工作台"
        subtitle={`${user.display_name}，從一個好關鍵字開始，建立可重用的搜尋知識。`}
        actions={
          <button className="btn btn-primary" onClick={() => setCreating(true)}>
            <Plus className="size-4" /> 新增關鍵字
          </button>
        }
      />
      <section className="workbench-intro"><div><p className="eyebrow">COLLECT. COMPOSE. REFINE.</p><h2>每一個查詢，<br />都有值得留下的語法。</h2><Link to="/keywords" className="workbench-link">開啟語法庫 <span>↗</span></Link></div><div className="workbench-example"><span className="eyebrow">THE LANGUAGE OF SEARCH</span><p><code>關鍵字 <b>&amp;</b> 洞察<br />(收集 <b>|</b> 重用) <b>!</b> 雜訊</code></p><Link to="/guide">四個運算子，組出精準查詢 →</Link></div></section>
      <Link to="/inspect" className="anatomy-invitation"><div><span className="eyebrow">NEW / QUERY ANATOMY</span><h2>一句語法，拆成可理解的條件。</h2><p>載入收藏的查詢 → 看群組與排除 → 試寫並另存新版本</p></div><div className="invitation-branches" aria-hidden="true"><span>查詢</span><b>├─ <i>茶 | 咖啡</i></b><b>└─ <em>NOT 業配</em></b></div><span className="invitation-arrow">↗</span></Link>
      {error && <ErrorBanner message={error} />}
      {loading && !data && <Loading />}
      {data && (
        <div className="flex flex-col gap-6">
          <div className="vault-stats grid grid-cols-2 lg:grid-cols-4">
            <StatCard icon={Library} label="看得到的關鍵字" value={data.total} note="自己的加上別人分享的" to="/keywords" />
            <StatCard
              icon={KeyRound}
              label="我的關鍵字"
              value={data.mine}
              note={`私人 ${data.mine_private}・分享 ${data.mine_shared}`}
              to="/keywords?scope=mine"
            />
            <StatCard icon={Combine} label="組合關鍵字" value={data.combinations} to="/keywords?kind=combination" />
            <StatCard icon={Star} label="我的收藏" value={data.favorites} to="/keywords?scope=favorites" />
          </div>

          <div className="grid grid-cols-1 gap-5 lg:grid-cols-5">
            <section className="card p-5 lg:col-span-3">
              <h2 className="mb-4 font-semibold">每月新增</h2>
              <MonthlyChart monthly={data.monthly} />
            </section>
            <div className="lg:col-span-2">
              <TodayCard initial={data.today} />
            </div>
          </div>

          <div className="grid grid-cols-1 gap-5 lg:grid-cols-5">
            <section className="card lg:col-span-3">
              <div className="flex items-center justify-between px-5 pt-5 pb-2">
                <h2 className="font-semibold">最近更新</h2>
                <Link to="/keywords" className="text-sm text-brand-700 hover:underline dark:text-brand-300">
                  看全部
                </Link>
              </div>
              {data.recent.length === 0 ? (
                <Empty icon={KeyRound} title="還沒有關鍵字">
                  按右上角的「新增關鍵字」寫第一個。
                </Empty>
              ) : (
                <ul className="divide-y divide-zinc-100 dark:divide-zinc-800">
                  {data.recent.map((k) => (
                    <li key={k.id}>
                      <Link to={`/keywords/${k.id}`} className="block px-5 py-3 hover:bg-zinc-50 dark:hover:bg-zinc-800/50">
                        <span className="flex flex-wrap items-center gap-2">
                          <span className="font-medium">{k.theme}</span>
                          <KeywordBadges keyword={k} />
                          <span className="ml-auto text-xs text-zinc-500 dark:text-zinc-400">
                            {k.owner.display_name}・{timeAgo(k.updated_at)}
                          </span>
                        </span>
                        <span className="mt-1 block truncate font-mono text-xs text-zinc-500 dark:text-zinc-400">{k.expression}</span>
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
            </section>
            <section className="card p-5 lg:col-span-2">
              <h2 className="mb-4 font-semibold">誰寫得最多</h2>
              {data.contributors.length === 0 ? (
                <p className="text-sm text-zinc-500 dark:text-zinc-400">還沒有資料。</p>
              ) : (
                <ol className="flex flex-col gap-3">
                  {data.contributors.map((c) => (
                    <li key={c.name}>
                      <div className="mb-1 flex justify-between text-sm">
                        <span>{c.name}</span>
                        <span className="text-zinc-500 tabular-nums dark:text-zinc-400">{c.count}</span>
                      </div>
                      <div className="h-1.5 overflow-hidden rounded-full bg-zinc-100 dark:bg-zinc-800">
                        <div
                          className="h-full rounded-full bg-brand-500 dark:bg-brand-400"
                          style={{ width: `${(c.count / data.contributors[0].count) * 100}%` }}
                        />
                      </div>
                    </li>
                  ))}
                </ol>
              )}
            </section>
          </div>
        </div>
      )}
      <KeywordEditor
        open={creating}
        onClose={() => setCreating(false)}
        onSaved={(k) => {
          setCreating(false);
          navigate(`/keywords/${k.id}`);
        }}
      />
    </PageBody>
  );
}
