import { useState } from "react";
import { Expression } from "../components/Expression";
import { PageBody } from "../components/Layout";
import { CheckResultView, useCheck } from "../components/SyntaxCheck";
import { PageHeader } from "../components/ui";
import { useApp } from "../context";

const START = "(珍珠奶茶|(珍珠 鮮奶 ~))&手搖!廣告";

function rules(slop: number): { syntax: string; meaning: string; example: string }[] {
  return [
    { syntax: "a&b", meaning: "兩個都要出現", example: "電動車&補助" },
    { syntax: "a|b", meaning: "出現其中一個就算", example: "珍珠奶茶|波霸奶茶" },
    { syntax: "a!b", meaning: "有 a 但不能有 b", example: "蘋果!水果" },
    { syntax: "( )", meaning: "先算括號裡的。& 和 | 混用時一定要加括號", example: "(電動車|電動機車)&補助" },
    { syntax: "(a b ~)", meaning: `a 和 b 相距 ${slop} 個字以內`, example: "(珍珠 鮮奶 ~)" },
    { syntax: "(a b ~5)", meaning: "自己指定距離：相距 5 個字以內", example: "(颱風 停班 ~5)" },
    { syntax: "\\& \\| \\! \\~ \\( \\)", meaning: "前面加反斜線，就是這個字本身", example: "AT\\&T|M\\&M" },
  ];
}

const ERRORS = [
  { bad: "a&&b", why: "不可有連續的邏輯運算子" },
  { bad: "a|", why: "運算子後面要有東西" },
  { bad: "!a", why: "不能只有排除的條件" },
  { bad: "(a|b", why: "括號沒有成對" },
  { bad: "(a)b", why: "括號和詞之間少了運算子" },
  { bad: "(a|b)~", why: "~ 只能接在詞的後面" },
];

/** 語法說明，加上可以直接試的練習區。 */
export function Guide() {
  const { meta } = useApp();
  const [expression, setExpression] = useState(START);
  const [precise, setPrecise] = useState(true);
  const check = useCheck(expression, precise);

  return (
    <PageBody>
      <PageHeader title="語法說明" subtitle="幾個符號，就能寫出送給 Elasticsearch / OpenSearch 的搜尋條件。" />

      <section className="card mb-4 p-4 sm:p-5">
        <h2 className="mb-1 text-sm font-semibold">試試看</h2>
        <p className="mb-3 text-sm text-zinc-500 dark:text-zinc-400">邊打邊檢查，下面是實際送出去的 query_string。</p>
        <label className="sr-only" htmlFor="guide-expression">
          關鍵字語法
        </label>
        <textarea
          id="guide-expression"
          className="input expr min-h-24"
          spellCheck={false}
          value={expression}
          onChange={(e) => setExpression(e.target.value)}
        />
        <label className="mt-2 mb-3 flex w-fit items-center gap-2 text-sm">
          <input
            type="checkbox"
            className="size-4 accent-brand-600"
            checked={precise}
            onChange={(e) => setPrecise(e.target.checked)}
          />
          精準比對
          <span className="text-zinc-500 dark:text-zinc-400">（每個詞加上引號；關掉的話 * 這類萬用字元才有作用）</span>
        </label>
        <CheckResultView {...check} />
        {check.result?.valid && check.result.terms.length > 0 && (
          <div className="mt-3 flex flex-wrap items-center gap-1.5 text-xs">
            <span className="text-zinc-500 dark:text-zinc-400">用到的詞</span>
            {check.result.terms.map((t, i) => (
              <span key={i} className="rounded-md border border-zinc-200 px-1.5 py-0.5 font-mono dark:border-zinc-700">
                {t}
              </span>
            ))}
          </div>
        )}
      </section>

      <section className="card mb-4 overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr className="border-b border-zinc-100 dark:border-zinc-800">
              <th className="th">寫法</th>
              <th className="th">意思</th>
              <th className="th">例子</th>
              <th className="th" />
            </tr>
          </thead>
          <tbody className="divide-y divide-zinc-100 dark:divide-zinc-800">
            {rules(meta?.default_slop ?? 20).map((r) => (
              <tr key={r.syntax}>
                <td className="td font-mono whitespace-nowrap">{r.syntax}</td>
                <td className="td">{r.meaning}</td>
                <td className="td">
                  <Expression value={r.example} />
                </td>
                <td className="td text-right">
                  <button className="btn btn-ghost px-2 py-1 text-xs" onClick={() => setExpression(r.example)}>
                    試這個
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <div className="grid gap-4 md:grid-cols-2">
        <section className="card p-4 sm:p-5">
          <h2 className="mb-3 text-sm font-semibold">常見的錯</h2>
          <ul className="flex flex-col gap-2 text-sm">
            {ERRORS.map((e) => (
              <li key={e.bad} className="flex items-baseline gap-3">
                <button
                  className="w-20 shrink-0 rounded bg-rose-50 px-1.5 py-0.5 text-left font-mono text-xs text-rose-800 hover:bg-rose-100 dark:bg-rose-950/60 dark:text-rose-300"
                  title="放到練習區"
                  onClick={() => setExpression(e.bad)}
                >
                  {e.bad}
                </button>
                <span className="text-zinc-600 dark:text-zinc-300">{e.why}</span>
              </li>
            ))}
          </ul>
        </section>
        <section className="card p-4 text-sm sm:p-5">
          <h2 className="mb-3 font-semibold">其他要知道的</h2>
          <ul className="flex list-disc flex-col gap-2 pl-5 text-zinc-600 dark:text-zinc-300">
            <li>全形的 ＆ ｜ ！ （ ） 會自動當成半形，從文件貼過來不用自己改。</li>
            <li>詞裡面的空白會留著：「紅茶 拿鐵」是一個詞，要兩個字相鄰才算。</li>
            <li>
              運算子照寫的順序送出去，不會自動幫你排優先順序。<code>a|b&c</code> 請寫成 <code>(a|b)&c</code> 或{" "}
              <code>a|(b&c)</code>。
            </li>
            <li>
              組合關鍵字用編號把別的關鍵字接起來，例如 <code>(#3|#5)!#8</code>
              ；每個關鍵字展開時會自己包一層括號。
            </li>
          </ul>
        </section>
      </div>
    </PageBody>
  );
}
