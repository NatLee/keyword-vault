import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { expect, test } from "vitest";
import { ToastProvider } from "../components/ui";
import { fakeApi } from "../test/fakeApi";
import type { Keyword } from "../types";
import { Keywords } from "./Keywords";

const keyword: Keyword = {
  id: 7,
  theme: "手搖飲",
  expression: "珍珠奶茶|(珍珠 鮮奶 ~)",
  logic: null,
  is_combination: false,
  visibility: "public",
  owner: { id: 1, username: "nat", display_name: "Nat" },
  tags: ["飲料"],
  favorite: false,
  comment_count: 2,
  created_at: "2026-01-05T08:00:00Z",
  updated_at: "2026-01-05T08:00:00Z",
  can_edit: true,
  can_delete: true,
};

function renderPage(url = "/keywords") {
  const queries: string[] = [];
  const calls = fakeApi({
    "GET /keywords": (_body, u) => {
      queries.push(u.search);
      return { items: [keyword], total: 1, page: 1, page_size: 20 };
    },
    "GET /tags": () => [{ name: "飲料", count: 1 }],
    "PUT /keywords/7/favorite": () => ({ favorite: true }),
  });
  render(
    <MemoryRouter initialEntries={[url]}>
      <ToastProvider>
        <Keywords />
      </ToastProvider>
    </MemoryRouter>,
  );
  return { calls, queries };
}

test("網址上的篩選條件會帶進查詢", async () => {
  const { queries } = renderPage("/keywords?scope=mine&tag=飲料&kind=plain");
  expect(await screen.findByText("手搖飲")).toBeInTheDocument();
  const params = new URLSearchParams(queries[0]);
  expect(params.get("scope")).toBe("mine");
  expect(params.get("tag")).toBe("飲料");
  expect(params.get("kind")).toBe("plain");
  expect(screen.getByRole("tab", { name: "我的" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
});

test("換範圍會重新查詢，收藏會馬上反映在畫面上", async () => {
  const { calls, queries } = renderPage();
  await screen.findByText("手搖飲");

  await userEvent.click(screen.getByRole("tab", { name: "收藏" }));
  await waitFor(() =>
    expect(
      queries.some((q) => new URLSearchParams(q).get("scope") === "favorites"),
    ).toBe(true),
  );

  await userEvent.click(screen.getByRole("button", { name: "收藏" }));
  expect(
    await screen.findByRole("button", { name: "取消收藏" }),
  ).toBeInTheDocument();
  expect(
    calls.some((c) => c.method === "PUT" && c.path === "/keywords/7/favorite"),
  ).toBe(true);
});

test("典藏卡和清單使用相同篩選，卡片可直接進入原語法草稿", async () => {
  const { queries } = renderPage("/keywords?tag=飲料&scope=mine");
  await screen.findByText("手搖飲");
  expect(
    screen.getByRole("link", { name: "拆解與試寫：手搖飲" }),
  ).toHaveAttribute("href", "/inspect?keyword=7");
  await userEvent.click(screen.getByRole("button", { name: "清單" }));
  expect(screen.getByRole("button", { name: "清單" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  expect(document.querySelector(".catalogue")).toHaveClass("list-view");
  expect(screen.getByRole("tab", { name: "我的" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  expect(queries).toHaveLength(1);
  await userEvent.click(screen.getByRole("button", { name: "清除主題篩選" }));
  await waitFor(() =>
    expect(
      queries.some((q) => {
        const p = new URLSearchParams(q);
        return !p.has("tag") && p.get("scope") === "mine";
      }),
    ).toBe(true),
  );
});

test("主題索引包含第十二筆之後的標籤並使用 API 篩選", async () => {
  const queries: string[] = [];
  fakeApi({
    "GET /keywords": (_body, u) => {
      queries.push(u.search);
      return { items: [keyword], total: 1, page: 1, page_size: 20 };
    },
    "GET /tags": () =>
      Array.from({ length: 14 }, (_, i) => ({
        name: `主題${i + 1}`,
        count: i + 1,
      })),
  });
  render(
    <MemoryRouter initialEntries={["/keywords"]}>
      <ToastProvider>
        <Keywords />
      </ToastProvider>
    </MemoryRouter>,
  );
  await userEvent.click(await screen.findByRole("button", { name: /主題14/ }));
  await waitFor(() =>
    expect(
      queries.some((q) => new URLSearchParams(q).get("tag") === "主題14"),
    ).toBe(true),
  );
  expect(screen.getByRole("button", { name: /主題14/ })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});
