import { expect, test } from "vitest";
import { authorizeNext } from "./access";
test("login return accepts only a relative local OAuth authorize request", () => {
  expect(
    authorizeNext(
      "?next=" + encodeURIComponent("/oauth/authorize?client_id=test&state=s"),
    ),
  ).toBe("/oauth/authorize?client_id=test&state=s");
  for (const value of [
    window.location.origin + "/oauth/authorize?client_id=test",
    "/oauth/authorize",
    "//evil.example/oauth/authorize?x=1",
    "/oauth/authorize?x=1#fragment",
    "/oauth/authorize?x=\\evil",
  ])
    expect(authorizeNext("?next=" + encodeURIComponent(value))).toBeNull();
});
