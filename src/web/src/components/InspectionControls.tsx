import { useEffect, useState } from "react";
import { Download } from "lucide-react";
import { api } from "../api";
import { errorText, useDebounced } from "../lib/hooks";
import type { InspectionArtifact, InspectionResult, InspectInput } from "../types";

export function useInspection(input: InspectInput, enabled: boolean) {
  const key = JSON.stringify(input);
  const debounced = useDebounced(key, 250);
  const [settled, setSettled] = useState<{key:string; result:InspectionResult|null; failure:string|null}|null>(null);
  useEffect(() => {
    const snapshot: InspectInput = JSON.parse(debounced);
    if (!snapshot.expression.trim() || !enabled) return;
    let stale = false;
    api.inspect(snapshot).then(result => {
      if (!stale) setSettled({key:debounced,result,failure:null});
    }).catch(error => {
      if (!stale) setSettled({key:debounced,result:null,failure:errorText(error)});
    });
    return () => { stale = true; };
  }, [debounced, enabled]);
  const current = enabled && input.expression.trim() && settled?.key === key ? settled : null;
  return {result:current?.result ?? null, failure:enabled ? current?.failure ?? null : "slop 請填入 0～4,294,967,295 的整數，或留空使用伺服器預設。", pending:enabled && Boolean(input.expression.trim()) && current === null};
}

export function exportInspection(artifact: InspectionArtifact) {
  const url = URL.createObjectURL(new Blob([JSON.stringify(artifact, null, 2)], {type:"application/json;charset=utf-8"}));
  const link = document.createElement("a");
  link.href = url;
  link.download = "keyword-inspection.json";
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function InspectionControls({precise,setPrecise,slop,setSlop,filter,setFilter,artifact,onApply}:{
  precise:boolean;setPrecise:(value:boolean)=>void;slop:string;setSlop:(value:string)=>void;
  filter:string;setFilter:(value:string)=>void;artifact:InspectionArtifact|null;onApply:()=>void;
}) {
  return <section className="inspection-controls" aria-label="編譯預覽設定">
    <div className="inspection-controls-row">
      <label className="inspection-mode"><input type="checkbox" checked={precise} onChange={e=>setPrecise(e.target.checked)}/> 精確片語模式</label>
      <label>預設片語 slop<input className="input" type="number" min="0" max="4294967295" step="1" value={slop} placeholder="伺服器預設" onChange={e=>setSlop(e.target.value)}/></label>
      <button type="button" className="btn btn-ghost" disabled={!artifact} onClick={()=>artifact && exportInspection(artifact)}><Download className="size-4"/> 匯出檢視 JSON</button>
    </div>
    <p className="hint">{precise ? "詞彙以引號包住，片語由目標搜尋引擎的分析器處理。" : "原樣模式會傳遞 Lucene 欄位與萬用字元語法。"}</p>
    <label>篩選語法（選填）<textarea className="input" rows={2} value={filter} onChange={e=>setFilter(e.target.value)} placeholder="例如：店家|咖啡館" spellCheck={false}/></label>
    <div className="inspection-controls-row"><p className="hint">兩段語法各自分組，再以 AND 結合。</p><button type="button" className="btn btn-ghost" disabled={!artifact?.filter} onClick={onApply}>套用篩選至草稿</button></div>
    {artifact && <p className="hint">本次預覽：{artifact.options.precise ? "精確片語" : "原樣"} · 預設 slop {artifact.options.default_slop}</p>}
  </section>;
}
