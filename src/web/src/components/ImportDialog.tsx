import { FileUp } from "lucide-react";
import { useEffect, useState } from "react";
import { api, ApiError } from "../api";
import { errorText } from "../lib/hooks";
import type { ImportProblem } from "../types";
import { Dialog, ErrorBanner, Spinner, useToast } from "./ui";

const TEMPLATE =
  "﻿主題,關鍵字,分享範圍,標籤\n手搖飲,珍珠奶茶|(珍珠 鮮奶 ~),公開,飲料;熱門\n電動車,(電動車|電動機車)&補助!二手,私人,交通\n";

/** 從 CSV 匯入一般關鍵字。有任何一列有問題就整批不匯入，並列出每一列的問題。 */
export function ImportDialog({ open, onClose, onImported }: { open: boolean; onClose: () => void; onImported: () => void }) {
  const toast = useToast();
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [problems, setProblems] = useState<ImportProblem[]>([]);

  useEffect(() => {
    if (!open) return;
    setFile(null);
    setError(null);
    setProblems([]);
  }, [open]);

  const submit = async () => {
    if (!file) return;
    setBusy(true);
    setError(null);
    setProblems([]);
    try {
      const { created } = await api.importCsv(file);
      toast(`匯入了 ${created} 個關鍵字`);
      onImported();
    } catch (e) {
      setError(errorText(e));
      const rows = e instanceof ApiError ? (e.data as { rows?: ImportProblem[] } | undefined)?.rows : undefined;
      setProblems(rows ?? []);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      wide
      title="從 CSV 匯入"
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            取消
          </button>
          <button type="button" className="btn btn-primary" disabled={!file || busy} onClick={() => void submit()}>
            {busy ? <Spinner /> : <FileUp className="size-4" />}
            匯入
          </button>
        </>
      }
    >
      <div className="flex flex-col gap-4 text-sm">
        <p className="text-zinc-600 dark:text-zinc-300">
          第一列是欄位名稱：<code>主題</code>、<code>關鍵字</code>，另外可以有 <code>分享範圍</code>（私人、群組、公開，沒寫是私人）和{" "}
          <code>標籤</code>（用 <code>;</code> 分開）。每一列的語法都會先檢查，有一列不對就整批不匯入。
        </p>
        <a
          className="w-fit text-brand-700 hover:underline dark:text-brand-300"
          href={`data:text/csv;charset=utf-8,${encodeURIComponent(TEMPLATE)}`}
          download="keywords-template.csv"
        >
          下載範例檔
        </a>
        <div>
          <label className="label" htmlFor="import-file">
            CSV 檔
          </label>
          <input
            id="import-file"
            type="file"
            accept=".csv,text/csv"
            className="input file:mr-3 file:rounded-md file:border-0 file:bg-brand-50 file:px-2.5 file:py-1 file:text-sm file:font-medium file:text-brand-700 dark:file:bg-brand-950 dark:file:text-brand-200"
            onChange={(e) => setFile(e.target.files?.[0] ?? null)}
          />
        </div>
        {error && <ErrorBanner message={error} />}
        {problems.length > 0 && (
          <div className="max-h-56 overflow-y-auto rounded-xl border border-zinc-200 dark:border-zinc-800">
            <table className="w-full">
              <thead className="sticky top-0 bg-zinc-50 dark:bg-zinc-900">
                <tr>
                  <th className="th w-14">列</th>
                  <th className="th">主題</th>
                  <th className="th">問題</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-zinc-100 dark:divide-zinc-800">
                {problems.map((p) => (
                  <tr key={p.line}>
                    <td className="td tabular-nums">{p.line}</td>
                    <td className="td">{p.theme}</td>
                    <td className="td text-rose-700 dark:text-rose-300">{p.message}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </Dialog>
  );
}
