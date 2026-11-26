import { CircleAlert, CircleCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../api";
import { errorText, useDebounced } from "../lib/hooks";
import type { CheckResult } from "../types";
import { CopyButton } from "./Expression";
import { Spinner } from "./ui";

/** 邊打邊檢查語法。`expression` 是空的時候不檢查。 */
export function useCheck(expression: string, precise = true) {
  const debounced = useDebounced(expression, 250);
  const [settled, setSettled] = useState<{
    expression: string;
    precise: boolean;
    result: CheckResult | null;
    failure: string | null;
  } | null>(null);

  useEffect(() => {
    if (!debounced.trim()) {
      setSettled(null);
      return;
    }
    let stale = false;
    api
      .check(debounced, precise)
      .then((r) => {
        if (stale) return;
        setSettled({
          expression: debounced,
          precise,
          result: r,
          failure: null,
        });
      })
      .catch((e) => {
        if (stale) return;
        setSettled({
          expression: debounced,
          precise,
          result: null,
          failure: errorText(e),
        });
      });
    return () => {
      stale = true;
    };
  }, [debounced, precise]);

  const current =
    expression.trim() &&
    settled?.expression === expression &&
    settled.precise === precise
      ? settled
      : null;
  return {
    result: current?.result ?? null,
    failure: current?.failure ?? null,
    pending: Boolean(expression.trim()) && current === null,
  };
}

/** 檢查結果：正確時顯示送給搜尋引擎的 query_string，錯誤時說哪裡錯。 */
export function CheckResultView({
  result,
  failure,
  pending,
}: {
  result: CheckResult | null;
  failure: string | null;
  pending: boolean;
}) {
  if (failure) {
    return (
      <p className="flex items-center gap-1.5 text-sm text-rose-700 dark:text-rose-300">
        <CircleAlert className="size-4 shrink-0" /> {failure}
      </p>
    );
  }
  if (!result) {
    return pending ? (
      <p className="flex items-center gap-1.5 text-sm text-zinc-500">
        <Spinner /> 檢查中…
      </p>
    ) : null;
  }
  if (!result.valid) {
    return (
      <p
        role="status"
        className="flex items-center gap-1.5 text-sm text-rose-700 dark:text-rose-300"
      >
        <CircleAlert className="size-4 shrink-0" />
        {result.error.message}（第 {result.error.position + 1} 個字元）
      </p>
    );
  }
  return (
    <div role="status" className={pending ? "opacity-60" : ""}>
      <div className="flex items-center justify-between gap-2">
        <p className="flex items-center gap-1.5 text-sm text-emerald-700 dark:text-emerald-400">
          <CircleCheck className="size-4 shrink-0" /> 語法正確，會送出這段
          query_string
        </p>
        <CopyButton text={result.query_string} />
      </div>
      <code className="expr mt-1 block rounded-lg bg-zinc-100 px-3 py-2 text-zinc-700 dark:bg-zinc-950 dark:text-zinc-300">
        {result.query_string}
      </code>
    </div>
  );
}
