import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { expect, test } from "vitest";
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
    "POST /tools/check": (body) => ({
      valid: true,
      query_string: (body as { expression: string }).expression,
      terms: ["tea"],
      clauses: [],
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
    expect(screen.getByRole("button", { name: "另存私人語法" })).toBeEnabled(),
  );
  await userEvent.click(screen.getByRole("button", { name: "另存私人語法" }));
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
    "POST /tools/check": () => ({
      valid: false,
      error: { message: "語法錯誤", position: 2 },
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
  expect(screen.getByRole("button", { name: "另存私人語法" })).toBeDisabled();
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
  expect(screen.getByRole("button", { name: "另存私人語法" })).toBeDisabled();
  expect(
    screen.queryByRole("button", { name: /節點/ }),
  ).not.toBeInTheDocument();
});
