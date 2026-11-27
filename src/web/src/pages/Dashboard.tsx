import { ArrowUpRight, Combine, Plus, Shuffle } from "lucide-react";
import { useState } from "react";
import { Link, useNavigate } from "react-router";
import { api } from "../api";
import { CopyButton, Expression } from "../components/Expression";
import { KeywordBadges } from "../components/KeywordBits";
import { KeywordEditor } from "../components/KeywordEditor";
import { PageBody } from "../components/Layout";
import { ErrorBanner, Loading, PageHeader } from "../components/ui";
import { useUser } from "../context";
import { timeAgo } from "../lib/format";
import { useAsync } from "../lib/hooks";
import type { Keyword } from "../types";

function ReadingCard({ initial }: { initial: Keyword | null }) {
  const [keyword, setKeyword] = useState(initial);
  const [busy, setBusy] = useState(false);
  const shuffle = async () => {
    setBusy(true);
    try {
      const next = await api.randomKeyword();
      if (next) setKeyword(next);
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="reading-card">
      <div className="archive-section-label">
        <span>READING COPY / 隨手翻閱</span>
        <button
          className="btn btn-ghost text-xs"
          disabled={busy}
          onClick={() => void shuffle().catch(() => {})}
        >
          <Shuffle className="size-4" /> 換一個
        </button>
      </div>
      {keyword ? (
        <>
          <div className="reading-number">
            NO. {String(keyword.id).padStart(3, "0")}
          </div>
          <Link className="reading-title" to={`/keywords/${keyword.id}`}>
            {keyword.theme}
          </Link>
          <div className="my-3 flex gap-2">
            <KeywordBadges keyword={keyword} />
          </div>
          <div className="reading-expression">
            <Expression value={keyword.expression} />
          </div>
          <div className="reading-footer">
            <span>{keyword.owner.display_name} 典藏</span>
            <CopyButton text={keyword.expression} label="複製語法" />
          </div>
          <Link
            className="reading-inspect"
            to={`/inspect?keyword=${keyword.id}`}
          >
            開啟這張語法的推敲桌 <ArrowUpRight className="size-4" />
          </Link>
        </>
      ) : (
        <p className="hint mt-6">把語法設為群組或公開，就能在這裡翻閱。</p>
      )}
    </section>
  );
}

export function Dashboard() {
  const user = useUser();
  const navigate = useNavigate();
  const { data, error, loading } = useAsync(() => api.stats(), []);
  const tags = useAsync(() => api.tags(), []);
  const [creating, setCreating] = useState(false);
  return (
    <PageBody>
      <PageHeader
        title="語法工作台"
        subtitle={`${user.display_name} 的典藏閱覽室`}
        actions={
          <button className="btn btn-primary" onClick={() => setCreating(true)}>
            <Plus className="size-4" /> 新增關鍵字
          </button>
        }
      />
      <section className="cabinet-hero">
        <div className="cabinet-story">
          <span className="eyebrow">THE QUERY CABINET / 關鍵字典藏室</span>
          <h2>
            值得留下的語法，
            <br />
            都有自己的位置。
          </h2>
          <p>
            把一次靈感，編入團隊的長期記憶。
            <br />
            依主題尋找，拆開理解，再寫成下一個好查詢。
          </p>
          <Link to="/keywords" className="cabinet-open">
            打開語法庫 <ArrowUpRight className="size-4" />
          </Link>
          <span className="cabinet-access">個人與團隊共用的搜尋知識典藏</span>
        </div>
        <div className="cabinet-inventory">
          <div className="cabinet-plaque">
            <span>KEYWORD VAULT</span>
            <strong>藏書目錄</strong>
            <small>ACCESSIBLE COLLECTION</small>
          </div>
          {data ? (
            <div className="cabinet-drawers">
              {[
                {
                  no: "01",
                  label: "全部館藏",
                  value: data.total,
                  to: "/keywords",
                  note: "你有權限讀取的語法",
                },
                {
                  no: "02",
                  label: "我的手稿",
                  value: data.mine,
                  to: "/keywords?scope=mine",
                  note: `私人 ${data.mine_private}・分享 ${data.mine_shared}`,
                },
                {
                  no: "03",
                  label: "組合索引",
                  value: data.combinations,
                  to: "/keywords?kind=combination",
                  note: "以編號引用既有語法",
                },
                {
                  no: "04",
                  label: "書籤收藏",
                  value: data.favorites,
                  to: "/keywords?scope=favorites",
                  note: "留給下次的好查詢",
                },
              ].map((d) => (
                <Link key={d.no} to={d.to} className="cabinet-drawer">
                  <small>{d.no}</small>
                  <div className="drawer-label">
                    <strong>{d.label}</strong>
                    <span>{d.note}</span>
                  </div>
                  <b>{String(d.value).padStart(2, "0")}</b>
                  <span className="drawer-pull" aria-hidden="true" />
                </Link>
              ))}
            </div>
          ) : (
            <div className="p-6">
              <Loading />
            </div>
          )}
        </div>
      </section>
      {error && <ErrorBanner message={error} />}
      {loading && !data && <Loading />}
      <section className="collection-section">
        <div className="archive-section-label">
          <span>SUBJECT DIVIDERS / 主題分冊</span>
          <Link to="/keywords">完整索引 ↗</Link>
        </div>
        {tags.error && <ErrorBanner message={tags.error} />}
        <div className="collection-folders">
          {tags.data?.slice(0, 6).map((tag, index) => (
            <Link
              key={tag.name}
              to={`/keywords?tag=${encodeURIComponent(tag.name)}`}
              className="collection-folder"
            >
              <span className="folder-tab">
                {String(index + 1).padStart(2, "0")} / SUBJECT
              </span>
              <strong>{tag.name}</strong>
              <span>
                {tag.count} 份語法 <ArrowUpRight className="size-4" />
              </span>
            </Link>
          ))}
        </div>
        {tags.data?.length === 0 && (
          <p className="hint">替語法加入標籤，主題分冊就會出現在這裡。</p>
        )}
      </section>
      {data && (
        <div className="archive-desk">
          <section className="accession-book">
            <div className="archive-section-label">
              <span>ACCESSION REGISTER / 最近修訂</span>
              <span>{data.recent.length} 份</span>
            </div>
            <div className="register-columns">
              <span>索引編號 / 主題</span>
              <span>修訂資訊</span>
            </div>
            {data.recent.length ? (
              data.recent.map((k) => (
                <Link
                  className="accession-entry"
                  to={`/keywords/${k.id}`}
                  key={k.id}
                >
                  <span className="accession-id">
                    {String(k.id).padStart(3, "0")}
                  </span>
                  <div>
                    <strong>{k.theme}</strong>
                    <code>{k.is_combination ? k.logic : k.expression}</code>
                  </div>
                  <span className="accession-meta">
                    {k.is_combination && <Combine className="size-4" />}
                    {k.owner.display_name}
                    <small>{timeAgo(k.updated_at)}更新</small>
                  </span>
                  <ArrowUpRight className="size-4" />
                </Link>
              ))
            ) : (
              <p className="p-5 text-sm">新增第一份語法，開始建立你的典藏。</p>
            )}
            <Link className="register-all" to="/keywords">
              翻閱全部語法 →
            </Link>
          </section>
          <ReadingCard initial={data.today} />
        </div>
      )}
      <Link to="/inspect" className="archive-workshop">
        <div className="workshop-mark" aria-hidden="true">
          ( &amp; )
        </div>
        <div>
          <span className="eyebrow">THE ANNOTATION DESK / 語法推敲桌</span>
          <h2>好語法，經得起拆開來看。</h2>
          <p>載入館藏 → 檢視結構 → 修改草稿 → 另存私人副本</p>
        </div>
        <ArrowUpRight className="size-6" />
      </Link>
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
