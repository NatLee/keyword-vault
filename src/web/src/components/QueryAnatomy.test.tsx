import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, test } from "vitest";
import type { Clause } from "../types";
import { QueryAnatomy } from "./QueryAnatomy";

export const clauses: Clause[] = [
  {
    parent: null,
    kind: "group",
    joiner: null,
    negated: true,
    text: null,
    slop: null,
    output: null,
  },
  {
    parent: 0,
    kind: "term",
    joiner: null,
    negated: false,
    text: "廣告",
    slop: null,
    output: '"廣告"',
  },
  {
    parent: 0,
    kind: "proximity",
    joiner: "OR",
    negated: false,
    text: "付費 內容",
    slop: 5,
    output: '"付費 內容"~5',
  },
];

test("節點檢視使用 API 的群組與 slop，不重新解析文字", async () => {
  render(<QueryAnatomy clauses={clauses} />);
  await userEvent.click(screen.getByRole("button", { name: /節點 3/ }));
  expect(screen.getByText('"付費 內容"~5')).toBeInTheDocument();
  expect(screen.getByText("位於排除群組內")).toBeInTheDocument();
  expect(screen.getByText("片語 slop = 5")).toBeInTheDocument();
});

test("大量節點有清楚的預覽上限", () => {
  render(
    <QueryAnatomy
      clauses={Array.from({ length: 130 }, () => ({
        ...clauses[1],
        parent: null,
      }))}
    />,
  );
  expect(screen.getAllByRole("button", { name: /節點/ })).toHaveLength(80);
  expect(screen.getByText(/顯示前 80 個，共 130 個節點/)).toBeInTheDocument();
});
