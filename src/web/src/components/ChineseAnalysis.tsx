import { useRef, useState } from "react";
import { api } from "../api";
import { errorText } from "../lib/hooks";
import type { AnalysisResult } from "../types";
import { CopyButton } from "./Expression";
import { ErrorBanner, Spinner } from "./ui";

export function ChineseAnalysis({expression,precise,enabled}:{expression:string;precise:boolean;enabled:boolean}) {
  const [mode,setMode]=useState<"normal"|"search">("normal");
  const key=JSON.stringify({expression,precise,mode});
  const latest=useRef(key); latest.current=key;
  const [state,setState]=useState<{key:string;pending:boolean;result:AnalysisResult|null;error:string|null}|null>(null);
  const current=enabled && precise && state?.key===key ? state : null;
  const tooLong=Array.from(expression).length>10000;
  const run=async()=>{
    const submitted=key;
    setState({key:submitted,pending:true,result:null,error:null});
    try {
      const result=await api.analyze({expression,precise,mode});
      if(latest.current===submitted) setState({key:submitted,pending:false,result,error:null});
    } catch(error) {
      if(latest.current===submitted) setState({key:submitted,pending:false,result:null,error:errorText(error)});
    }
  };
  return <section className="inspection-controls chinese-analysis" aria-label="中文斷詞預覽">
    <div className="inspection-controls-row"><h2>中文斷詞預覽</h2>
      <label>斷詞模式<select className="input" value={mode} onChange={e=>setMode(e.target.value as "normal"|"search")}><option value="normal">一般斷詞</option><option value="search">搜尋模式（含重疊詞）</option></select></label>
      <button type="button" className="btn btn-ghost" onClick={()=>void run()} disabled={!enabled||!precise||tooLong||current?.pending}>{current?.pending && <Spinner/>} 預覽中文斷詞</button>
    </div>
    <p className="hint">本機中文斷詞預覽；不代表 Elasticsearch 分析結果。只分析來源詞彙，原稿與編譯結果保持不變。</p>
    {!precise && <p className="hint">原樣模式不會進行中文斷詞；請切回精確片語模式。</p>}
    {tooLong && <p className="hint">中文斷詞預覽最多 10,000 個字元。</p>}
    {current?.error && <ErrorBanner message={current.error}/>}
    {current?.result && <>
      <p className="hint">{current.result.engine} {current.result.engine_version} · 詞典 {current.result.dictionary.id} · <code title={current.result.dictionary.sha256}>{current.result.dictionary.sha256.slice(0,12)}</code></p>
      <p className="hint">位置以每個正規化詞彙內的 Unicode 字元計算，從 0 開始，結束位置不包含在內。搜尋模式可能有重疊詞。</p>
      {current.result.leaves.map(leaf=><div className="analysis-leaf" key={leaf.clause}>
        <h3>節點 {leaf.clause+1} · {leaf.text}{leaf.negated ? " · 直接排除" : ""}</h3>
        <div className="analysis-tokens">{leaf.tokens.map((token,index)=><div className="analysis-token" key={`${token.start}-${token.end}-${index}`}><span>{token.text}</span><small>{token.start}–{token.end}</small><CopyButton label={`複製 ${token.text}`} text={token.text.replace(/[&|!~()\\]/g,"\\$&")}/></div>)}</div>
        {!leaf.tokens.length && <p className="hint">這個詞彙沒有可建議的文字詞元。</p>}
      </div>)}
    </>}
  </section>;
}
