import { Check, Copy } from "lucide-react";
import { useState } from "react";
import { tokenize, type Token } from "../lib/highlight";
import type { DiffSegment } from "../types";

const TOKEN_CLASS: Record<Token["kind"], string> = {
  term: "",
  operator: "px-1 font-semibold text-brand-600 dark:text-brand-300",
  paren: "text-zinc-400 dark:text-zinc-500",
  near: "font-semibold text-amber-600 dark:text-amber-400",
  ref: "rounded bg-sky-100 px-1 text-sky-800 dark:bg-sky-950 dark:text-sky-300",
};

/** 上了色的關鍵字語法。`clamp` 時最多顯示兩行。 */
export function Expression({ value, clamp }: { value: string; clamp?: boolean }) {
  return (
    <code className={`expr block ${clamp ? "line-clamp-2" : ""}`}>
      {tokenize(value).map((t, i) => (
        <span key={i} className={TOKEN_CLASS[t.kind]}>
          {t.text}
        </span>
      ))}
    </code>
  );
}

/** 兩段語法的差異：刪掉的畫紅色刪除線，加上的畫綠色。 */
export function DiffView({ segments }: { segments: DiffSegment[] }) {
  return (
    <code className="expr block">
      {segments.map((s, i) =>
        s.op === "equal" ? (
          <span key={i}>{s.text}</span>
        ) : s.op === "insert" ? (
          <ins key={i} className="rounded bg-emerald-100 text-emerald-900 no-underline dark:bg-emerald-950 dark:text-emerald-200">
            {s.text}
          </ins>
        ) : (
          <del key={i} className="rounded bg-rose-100 text-rose-900 dark:bg-rose-950 dark:text-rose-300">
            {s.text}
          </del>
        ),
      )}
    </code>
  );
}

export function CopyButton({ text, label = "複製" }: { text: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      // 瀏覽器不給用剪貼簿（例如不是 https）：讓使用者自己選取
    }
  };
  return (
    <button type="button" className="btn btn-ghost px-2 py-1 text-xs" onClick={() => void copy()}>
      {copied ? <Check className="size-4 text-emerald-600" /> : <Copy className="size-4" />}
      {copied ? "已複製" : label}
    </button>
  );
}
