import { ArrowLeftRight, Equal, GitCompareArrows } from "lucide-react";
import { useEffect, useState } from "react";
import { useSearchParams } from "react-router";
import { api } from "../api";
import { DiffView } from "../components/Expression";
import { PageBody } from "../components/Layout";
import { CheckResultView, useCheck } from "../components/SyntaxCheck";
import { Empty, ErrorBanner, PageHeader } from "../components/ui";
import { errorText, useDebounced } from "../lib/hooks";
import type { DiffResult } from "../types";

function Side({ id, label, value, onChange }: { id: string; label: string; value: string; onChange: (v: string) => void }) {
  const check = useCheck(value);
  return (
    <div className="card flex flex-col gap-3 p-4">
      <div>
        <label className="label" htmlFor={id}>
          {label}
        </label>
        <textarea
          id={id}
          className="input expr min-h-36"
          spellCheck={false}
          placeholder="貼上關鍵字語法"
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
      </div>
      <CheckResultView {...check} />
    </div>
  );
}

/** 比對兩段語法哪裡不一樣。`?a=…&b=…` 可以預先帶入。 */
export function Compare() {
  const [params] = useSearchParams();
  const [a, setA] = useState(params.get("a") ?? "");
  const [b, setB] = useState(params.get("b") ?? "");
  const pair = useDebounced(`${a}\u0000${b}`, 250);
  const [diff, setDiff] = useState<DiffResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const [left, right] = pair.split("\u0000");
    if (!left.trim() || !right.trim()) {
      setDiff(null);
      setError(null);
      return;
    }
    let stale = false;
    api
      .diff(left, right)
      .then((d) => {
        if (stale) return;
        setDiff(d);
        setError(null);
      })
      .catch((e) => {
        if (!stale) setError(errorText(e));
      });
    return () => {
      stale = true;
    };
  }, [pair]);

  return (
    <PageBody>
      <PageHeader
        title="語法比對"
        subtitle="兩段很長的語法到底差在哪裡。"
        actions={
          <button
            className="btn btn-secondary"
            onClick={() => {
              setA(b);
              setB(a);
            }}
          >
            <ArrowLeftRight className="size-4" /> 左右對調
          </button>
        }
      />
      <div className="grid gap-4 md:grid-cols-2">
        <Side id="compare-a" label="原本的" value={a} onChange={setA} />
        <Side id="compare-b" label="新的" value={b} onChange={setB} />
      </div>
      <section className="card mt-4">
        <h2 className="border-b border-zinc-100 px-4 py-3 text-sm font-semibold sm:px-5 dark:border-zinc-800">差異</h2>
        {error ? (
          <div className="p-4">
            <ErrorBanner message={error} />
          </div>
        ) : !diff ? (
          <Empty icon={GitCompareArrows} title="兩邊都貼上語法就會開始比對">
            紅色刪除線是原本有、新的沒有的字；綠色是新加的字。
          </Empty>
        ) : diff.same ? (
          <Empty icon={Equal} title="兩段語法一模一樣" />
        ) : (
          <div className="p-4 sm:p-5">
            <DiffView segments={diff.segments} />
          </div>
        )}
      </section>
    </PageBody>
  );
}
