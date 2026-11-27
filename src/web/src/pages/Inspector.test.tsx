import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { expect, test, vi } from "vitest";
import { ToastProvider } from "../components/ui";
import { fakeApi } from "../test/fakeApi";
import { Inspector } from "./Inspector";

test("從既有語法載入草稿，驗證後另存私人副本", async () => {
  const saved = {
    id: 7,
    theme: "手搖飲",
    expression: "tea|coffee",
    tags: [],
    is_combination: false,
  };
  const calls = fakeApi({
    "GET /keywords": () => ({ items: [saved], total: 1 }),
    "GET /keywords/7": () => saved,
    "POST /tools/inspect": (body) => ({
      valid: true,
      inspection: {schema_version:1, expression:(body as {expression:string}).expression, filter_expression:null, options:{precise:true,default_slop:20}, query:{query_string:"tea",terms:["tea"],clauses:[]}, filter:null, query_string:"tea"},
    }),
    "POST /keywords": (body) => ({ ...(body as object), id: 9 }),
  });
  render(
    <MemoryRouter initialEntries={["/inspect?keyword=7"]}>
      <ToastProvider>
        <Inspector />
      </ToastProvider>
    </MemoryRouter>,
  );
  await waitFor(() =>
    expect(screen.getByLabelText("草稿語法")).toHaveValue("tea|coffee"),
  );
  await userEvent.type(screen.getByLabelText("草稿語法"), "&shop");
  await userEvent.clear(screen.getByLabelText("新語法名稱"));
  await userEvent.type(screen.getByLabelText("新語法名稱"), "飲料店研究");
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "另存來源語法" })).toBeEnabled(),
  );
  await userEvent.click(screen.getByRole("button", { name: "另存來源語法" }));
  expect(
    await screen.findByRole("link", { name: /開啟新語法/ }),
  ).toHaveAttribute("href", "/keywords/9");
  expect(
    calls.find((c) => c.method === "POST" && c.path === "/keywords")?.body,
  ).toEqual({
    theme: "飲料店研究",
    expression: "tea|coffee&shop",
    visibility: "private",
    tags: [],
  });
  expect(calls.some((c) => c.method === "PATCH")).toBe(false);
});

test("無效草稿不可儲存，也不顯示舊結構", async () => {
  fakeApi({
    "GET /keywords": () => ({ items: [], total: 0 }),
    "POST /tools/inspect": () => ({
      valid: false,
      error: { source:"expression", message: "語法錯誤", position: 2 },
    }),
  });
  render(
    <MemoryRouter>
      <ToastProvider>
        <Inspector />
      </ToastProvider>
    </MemoryRouter>,
  );
  expect(await screen.findByText(/語法錯誤/)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "另存來源語法" })).toBeDisabled();
  expect(
    screen.queryByRole("button", { name: /節點/ }),
  ).not.toBeInTheDocument();
});

test("無權讀取的語法不會帶出草稿內容", async () => {
  fakeApi({ "GET /keywords": () => ({ items: [], total: 0 }) });
  render(
    <MemoryRouter initialEntries={["/inspect?keyword=999"]}>
      <ToastProvider>
        <Inspector />
      </ToastProvider>
    </MemoryRouter>,
  );
  expect(
    await screen.findByText("no fake for GET /keywords/999"),
  ).toBeInTheDocument();
  expect(screen.getByLabelText("草稿語法")).toHaveValue("");
  expect(screen.getByRole("button", { name: "另存來源語法" })).toBeDisabled();
  expect(
    screen.queryByRole("button", { name: /節點/ }),
  ).not.toBeInTheDocument();
});

test("預覽選項與篩選不會默默改寫另存的來源", async () => {
  const calls = fakeApi({
    "GET /keywords": () => ({ items: [], total: 0 }),
    "POST /tools/inspect": (body) => ({valid:true, inspection:{schema_version:1, ...(body as object), options:{precise:false,default_slop:5},query:{query_string:"tea",terms:["tea"],clauses:[]},filter:{query_string:"shop OR cafe",terms:["shop","cafe"],clauses:[]},query_string:"(shop OR cafe) AND (tea)"}}),
    "POST /keywords": (body) => ({ ...(body as object), id: 12 }),
  });
  render(<MemoryRouter><ToastProvider><Inspector /></ToastProvider></MemoryRouter>);
  const source = screen.getByLabelText("草稿語法");
  await userEvent.clear(source);
  await userEvent.type(source, "tea");
  await userEvent.click(screen.getByLabelText("精確片語模式"));
  await userEvent.clear(screen.getByLabelText("預設片語 slop"));
  await userEvent.type(screen.getByLabelText("預設片語 slop"), "5");
  await userEvent.type(screen.getByLabelText("篩選語法（選填）"), "shop|cafe");
  await userEvent.type(screen.getByLabelText("新語法名稱"), "來源草稿");
  await waitFor(() => expect(screen.getByRole("button", {name:"另存來源語法"})).toBeEnabled());
  await userEvent.click(screen.getByRole("button", {name:"另存來源語法"}));
  expect(await screen.findByRole("link", {name:/開啟新語法/})).toHaveAttribute("href", "/keywords/12");
  expect(calls.find(c => c.method === "POST" && c.path === "/keywords")?.body).toEqual({theme:"來源草稿",expression:"tea",visibility:"private",tags:[]});
  expect(calls.filter(c => c.path === "/tools/inspect").at(-1)?.body).toEqual({expression:"tea",filter_expression:"shop|cafe",precise:false,default_slop:5});
  await userEvent.click(screen.getByRole("button", {name:"套用篩選至草稿"}));
  expect(source).toHaveValue("(shop|cafe)&(tea)");
  expect(screen.getByLabelText("篩選語法（選填）")).toHaveValue("");
});

test("過期預覽無法覆蓋新的結果，匯出只包含目前的完整 artifact", async () => {
  let finishOld: ((value: unknown) => void) | undefined;
  let exported: Blob | undefined;
  const make = (expression: string, output: string) => ({valid:true,inspection:{schema_version:1,expression,filter_expression:null,options:{precise:true,default_slop:20},query:{query_string:output,terms:[expression],clauses:[]},filter:null,query_string:output}});
  fakeApi({
    "GET /keywords":()=>({items:[],total:0}),
    "POST /tools/inspect":body=>(body as {expression:string}).expression === "new" ? make("new","CURRENT_QUERY") : new Promise(resolve=>{finishOld=resolve;}),
  });
  const NativeURL=URL;
  vi.stubGlobal("URL",class extends NativeURL {
    static createObjectURL(blob:Blob) { exported=blob; return "blob:inspection"; }
    static revokeObjectURL() {}
  });
  vi.spyOn(HTMLAnchorElement.prototype,"click").mockImplementation(()=>{});
  render(<MemoryRouter><ToastProvider><Inspector/></ToastProvider></MemoryRouter>);
  await waitFor(()=>expect(finishOld).toBeDefined());
  await userEvent.clear(screen.getByLabelText("草稿語法"));
  expect(screen.getByRole("button",{name:"匯出檢視 JSON"})).toBeDisabled();
  await userEvent.type(screen.getByLabelText("草稿語法"),"new");
  expect(await screen.findByText("CURRENT_QUERY")).toBeInTheDocument();
  finishOld!(make("old","STALE_QUERY"));
  await userEvent.click(screen.getByRole("button",{name:"匯出檢視 JSON"}));
  const jsonText=await new Promise<string>((resolve,reject)=>{const reader=new FileReader();reader.onload=()=>resolve(String(reader.result));reader.onerror=reject;reader.readAsText(exported!);});
  expect(JSON.parse(jsonText)).toEqual(make("new","CURRENT_QUERY").inspection);
  expect(screen.queryByText("STALE_QUERY")).not.toBeInTheDocument();
});
