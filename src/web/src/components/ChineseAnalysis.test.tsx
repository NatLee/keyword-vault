import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { test, expect } from "vitest";
import { fakeApi } from "../test/fakeApi";
import { ChineseAnalysis } from "./ChineseAnalysis";

test("中文預覽顯示詞元與詞典，切換來源立即清除結果", async () => {
  const calls=fakeApi({"POST /tools/analyze":()=>({engine:"jieba-rs",engine_version:"0.11.0",mode:"normal",dictionary:{id:"tc-test",sha256:"abcd"},offset_unit:"unicode_scalar",leaves:[{clause:0,text:"臺積電",negated:false,tokens:[{text:"臺積電",start:0,end:3}]}]})});
  const view=render(<ChineseAnalysis expression="臺積電" precise={true} enabled={true}/>);
  await userEvent.click(screen.getByRole("button",{name:"預覽中文斷詞"}));
  expect(await screen.findByRole("button",{name:"複製 臺積電"})).toBeInTheDocument();
  expect(screen.getByText(/tc-test/)).toBeInTheDocument();
  expect(calls[0].body).toEqual({expression:"臺積電",mode:"normal",precise:true});
  view.rerender(<ChineseAnalysis expression="台灣高鐵" precise={true} enabled={true}/>);
  await waitFor(()=>expect(screen.queryByRole("button",{name:"複製 臺積電"})).not.toBeInTheDocument());
});

test("原樣模式明確停用斷詞，不傳送 Lucene 字串", async () => {
  const calls=fakeApi({});
  render(<ChineseAnalysis expression="title:tea*" precise={false} enabled={true}/>);
  expect(screen.getByRole("button",{name:"預覽中文斷詞"})).toBeDisabled();
  expect(screen.getByText(/原樣模式不會進行中文斷詞/)).toBeInTheDocument();
  expect(calls).toHaveLength(0);
});
