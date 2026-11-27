import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { expect, test } from "vitest";
import { App } from "../App";
import { AppProvider } from "../context";
import { ToastProvider } from "../components/ui";
import { fakeApi } from "../test/fakeApi";
const user = {
  id: 2,
  username: "reader",
  display_name: "Reader",
  role: "member",
};
const zero = { attempts: 0, successes: 0, failures: 0, pending: 0 };
const common = {
  "GET /meta": () => ({ version: "1.0.0", default_slop: 20 }),
  "GET /auth/state": () => ({ setup_required: false, user }),
  "GET /stats": () => ({
    total: 0,
    mine: 0,
    mine_private: 0,
    mine_shared: 0,
    combinations: 0,
    favorites: 0,
    monthly: [],
    contributors: [],
    recent: [],
    today: null,
  }),
  "GET /access/providers": () => ({
    items: [
      {
        provider: "github",
        name: "GitHub",
        enabled: false,
        login_start_url: "/oauth/login/github",
        disabled_reason: "Provider credentials are not configured",
      },
      {
        provider: "google",
        name: "Google",
        enabled: false,
        login_start_url: "/oauth/login/google",
        disabled_reason: "Provider credentials are not configured",
      },
    ],
  }),
  "GET /access/overview": () => ({
    identities: [],
    credentials: [],
    grants: [],
    summary: { ...zero, days: 7, timezone: "UTC" },
  }),
  "GET /access/usage": () => ({
    days: 7,
    from: "2026-11-21T00:00:00Z",
    to: "2026-11-28T00:00:00Z",
    timezone: "UTC",
    totals: zero,
    daily: [{ date: "2026-11-27", ...zero }],
  }),
  "GET /access/events": () => ({ items: [], next_cursor: null }),
};
function app(path: string) {
  render(
    <MemoryRouter initialEntries={[path]}>
      <AppProvider>
        <ToastProvider>
          <App />
        </ToastProvider>
      </AppProvider>
    </MemoryRouter>,
  );
}
test("personal credential is shown once and closing removes it from the page", async () => {
  const calls = fakeApi({
    ...common,
    "POST /access/credentials": () => ({
      credential: {
        id: "key1",
        name: "Research",
        prefix: "ak_prefix",
        scopes: ["keywords:read", "expressions:read"],
        audiences: ["api", "mcp"],
        created_at: "2026-11-27T00:00:00Z",
        expires_at: "2026-12-27T00:00:00Z",
        revoked_at: null,
        last_used_at: null,
      },
      token: "ONE-TIME-TEST-KEY",
    }),
  });
  app("/connections");
  expect(
    await screen.findByRole("heading", { name: "連線與存取" }),
  ).toBeInTheDocument();
  await userEvent.type(screen.getByLabelText("金鑰名稱"), "Research");
  await userEvent.click(screen.getByRole("button", { name: "建立 API 金鑰" }));
  expect(await screen.findByText("ONE-TIME-TEST-KEY")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "關閉金鑰原值" })).toHaveFocus();
  expect(
    calls.find((c) => c.method === "POST" && c.path === "/access/credentials")
      ?.body,
  ).toEqual({
    name: "Research",
    scopes: ["keywords:read", "expressions:read"],
    audiences: ["api", "mcp"],
    expires_in_days: 30,
  });
  await userEvent.click(screen.getByRole("button", { name: "關閉金鑰原值" }));
  expect(screen.queryByText("ONE-TIME-TEST-KEY")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "建立 API 金鑰" })).toHaveFocus();
  expect(localStorage.getItem("access_token")).toBeNull();
  expect(screen.getByRole("button", { name: "綁定 GitHub" })).toBeDisabled();
  expect(screen.getByText("UTC · 最近 7 天")).toBeInTheDocument();
});
test("consent renders server immutable scopes and native approve/deny form", async () => {
  fakeApi({
    ...common,
    "GET /access/consents/pending1": () => ({
      request_id: "pending1",
      client_name: "Research Client",
      scopes: ["keywords:read"],
      resource: "https://vault.example/mcp",
      expires_at: "2026-11-27T12:00:00Z",
      user,
    }),
  });
  app("/consent?request_id=pending1");
  expect(
    await screen.findByRole("heading", { name: "應用程式授權" }),
  ).toBeInTheDocument();
  expect(await screen.findByText("Research Client")).toBeInTheDocument();
  const approve = screen.getByRole("button", { name: "同意授權" });
  const form = approve.closest("form")!;
  expect(form).toHaveAttribute("action", "/oauth/consent/pending1");
  expect(form).toHaveAttribute("method", "post");
  expect(approve).toHaveAttribute("name", "decision");
  expect(approve).toHaveAttribute("value", "approve");
  expect(screen.getByRole("button", { name: "拒絕" })).toHaveAttribute(
    "value",
    "deny",
  );
  expect(form.querySelector('[name="scopes"]')).toBeNull();
  await waitFor(() =>
    expect(screen.getByText("https://vault.example/mcp")).toBeInTheDocument(),
  );
});
