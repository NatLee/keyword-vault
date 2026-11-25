import { describe, expect, it } from "vitest";
import { tokenize } from "./highlight";
import { parseTags, timeAgo } from "./format";

const kinds = (s: string) => tokenize(s).map((t) => `${t.kind}:${t.text}`);

describe("tokenize", () => {
  it("splits operators, parentheses and proximity", () => {
    expect(kinds("(珍珠奶茶|(珍珠 鮮奶 ~5))&手搖!廣告")).toEqual([
      "paren:(",
      "term:珍珠奶茶",
      "operator:|",
      "paren:(",
      "term:珍珠 鮮奶 ",
      "near:~5",
      "paren:)",
      "paren:)",
      "operator:&",
      "term:手搖",
      "operator:!",
      "term:廣告",
    ]);
  });

  it("keeps escaped operators inside the term and marks references", () => {
    expect(kinds("AT\\&T|a")).toEqual(["term:AT\\&T", "operator:|", "term:a"]);
    expect(kinds("(#3｜#5)!#8")).toEqual(["paren:(", "ref:#3", "operator:｜", "ref:#5", "paren:)", "operator:!", "ref:#8"]);
  });

  it("always joins back to the input", () => {
    for (const s of ["", "a ~ 3 )", "(a b ~ )|c", "\\", "a\\", "~~", "＆（"]) {
      expect(tokenize(s).map((t) => t.text).join("")).toBe(s);
    }
  });
});

describe("format helpers", () => {
  it("parses tags from free text", () => {
    expect(parseTags(" #飲料, 熱門　飲料；茶 ")).toEqual(["飲料", "熱門", "茶"]);
    expect(parseTags("")).toEqual([]);
  });

  it("describes how long ago", () => {
    const now = Date.parse("2026-11-25T12:00:00Z");
    expect(timeAgo("2026-11-25T11:59:40Z", now)).toBe("剛剛");
    expect(timeAgo("2026-11-25T11:20:00Z", now)).toBe("40 分鐘前");
    expect(timeAgo("2026-11-25T02:00:00Z", now)).toBe("10 小時前");
    expect(timeAgo("2026-11-20T12:00:00Z", now)).toBe("5 天前");
  });
});
