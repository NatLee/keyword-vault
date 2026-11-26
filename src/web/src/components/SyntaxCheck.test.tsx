import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { api } from "../api";
import type { CheckResult } from "../types";
import { useCheck } from "./SyntaxCheck";

test("等待 API 時不把上一個草稿的結果當成目前結果", async () => {
  let finish!: (result: CheckResult) => void;
  vi.spyOn(api, "check").mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const { result, rerender } = renderHook(
    ({ expression }) => useCheck(expression),
    { initialProps: { expression: "tea" } },
  );
  expect(result.current.pending).toBe(true);
  await act(async () =>
    finish({ valid: true, query_string: '"tea"', terms: ["tea"], clauses: [] }),
  );
  expect(result.current.result?.valid).toBe(true);
  rerender({ expression: "tea&&coffee" });
  expect(result.current.result).toBeNull();
  expect(result.current.pending).toBe(true);
  await waitFor(() =>
    expect(api.check).toHaveBeenCalledWith("tea&&coffee", true),
  );
  expect(result.current.pending).toBe(true);
  await act(async () =>
    finish({ valid: false, error: { message: "連續運算子", position: 4 } }),
  );
  expect(result.current.pending).toBe(false);
  expect(result.current.result?.valid).toBe(false);
  vi.restoreAllMocks();
});
