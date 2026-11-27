import { useState } from "react";
import type { Clause } from "../types";

/** Render the server's AST. No client-side grammar or precedence is inferred. */
export function QueryAnatomy({
  clauses,
  compact = false,
}: {
  clauses: Clause[];
  compact?: boolean;
}) {
  const [selected, setSelected] = useState(0);
  const limit = compact ? 24 : 80;
  const visible = clauses.slice(0, limit);
  const index = Math.min(selected, visible.length - 1);
  const current = visible[index];
  const parents = (at: number) => {
    const chain: number[] = [];
    let parent = clauses[at]?.parent;
    while (parent !== null && parent !== undefined && parent < at) {
      chain.push(parent);
      at = parent;
      parent = clauses[at]?.parent;
    }
    return chain;
  };
  const depth = (at: number) => Math.min(parents(at).length, 5);
  const excludedParent =
    current && parents(index).some((i) => clauses[i].negated);
  const terms = clauses.filter((c) => c.kind !== "group").length;
  const groups = clauses.length - terms;
  if (!current) return <p className="hint">這段語法沒有可顯示的節點。</p>;
  return (
    <section
      className={`anatomy ${compact ? "anatomy-compact" : ""}`}
      aria-label="語法結構"
    >
      <div className="anatomy-heading">
        <span>EXPRESSION ANATOMY</span>
        <div>
          <b>{terms}</b> 詞彙 <i /> <b>{groups}</b> 群組 <i />{" "}
          <b>{clauses.filter((c) => c.negated).length}</b> 排除
        </div>
      </div>
      <div className="anatomy-layout">
        <div className="anatomy-diagram">
          <div className="anatomy-root">
            <span>●</span> 寫入的順序與括號 <small>點選節點查看</small>
          </div>
          <div className="anatomy-rows">
            <svg
              className="anatomy-wires"
              width="86"
              height={visible.length * 54}
              aria-hidden="true"
            >
              {visible.map((c, i) => {
                const x = 10 + depth(i) * 11;
                const parent = c.parent;
                const start =
                  parent === null
                    ? { x: 5, y: 0 }
                    : { x: 10 + depth(parent) * 11, y: parent * 54 + 27 };
                return (
                  <path
                    key={i}
                    d={`M ${start.x} ${start.y} V ${i * 54 + 27} H 82`}
                    className={i === index ? "active" : ""}
                    data-depth={x}
                  />
                );
              })}
            </svg>
            {visible.map((c, i) => (
              <button
                key={i}
                type="button"
                aria-label={`節點 ${i + 1}：${c.text ?? "括號群組"}`}
                aria-pressed={index === i}
                className={`anatomy-node ${c.kind} ${c.negated ? "excluded" : ""}`}
                onClick={() => setSelected(i)}
              >
                <span className="node-number">
                  {String(i + 1).padStart(2, "0")}
                </span>
                <span className="node-join">
                  {[c.joiner, c.negated ? "NOT" : null]
                    .filter(Boolean)
                    .join(" ") || "—"}
                </span>
                <span className="node-text">{c.text ?? "括號群組"}</span>
                <span className="node-kind">
                  {c.kind === "group"
                    ? `( ${parents(i).length + 1} )`
                    : c.kind === "proximity"
                      ? `~${c.slop}`
                      : "詞"}
                </span>
              </button>
            ))}
          </div>
          {clauses.length > limit && (
            <p className="anatomy-limit">
              顯示前 {limit} 個，共 {clauses.length}{" "}
              個節點。完整語法與編譯結果仍保留。
            </p>
          )}
        </div>
        <aside key={`${index}:${current.text ?? "group"}`} className="anatomy-note" aria-live="polite">
          <span className="eyebrow">
            CLAUSE / {String(index + 1).padStart(2, "0")}
          </span>
          <h3>{current.text ?? "把一組條件放在一起"}</h3>
          <p>
            {current.negated
              ? "直接排除此節點"
              : excludedParent
                ? "位於排除群組內"
                : "此節點沒有直接排除"}
          </p>
          <dl>
            <div>
              <dt>前接運算子</dt>
              <dd>{current.joiner ?? "起點 / 隱含連接"}</dd>
            </div>
            <div>
              <dt>所屬群組</dt>
              <dd>
                {current.parent === null
                  ? "最外層"
                  : `節點 ${current.parent + 1}`}
              </dd>
            </div>
          </dl>
          {current.kind === "proximity" && (
            <p className="anatomy-slop">片語 slop = {current.slop}</p>
          )}
          {current.output && <code>{current.output}</code>}
          <p className="anatomy-footnote">
            圖中只呈現實際括號與運算子順序。混用 &amp; 和 |
            時請加括號；搜尋引擎決定最終比對。
          </p>
        </aside>
      </div>
    </section>
  );
}
