/** 語法上色用的片段。 */
export interface Token {
  kind: "term" | "operator" | "paren" | "near" | "ref";
  text: string;
}

const NORMALISE: Record<string, string> = { "＆": "&", "｜": "|", "！": "!", "（": "(", "）": ")", "～": "~" };

/**
 * 把關鍵字語法（或組合方式）切成片段。只用來上色，不檢查語法；
 * 接起來一定等於原本的字串。
 */
export function tokenize(expression: string): Token[] {
  const tokens: Token[] = [];
  let term = "";
  const flush = () => {
    if (!term) return;
    tokens.push({ kind: /^\s*#\d+\s*$/.test(term) ? "ref" : "term", text: term });
    term = "";
  };
  const chars = Array.from(expression);
  for (let i = 0; i < chars.length; i++) {
    const c = chars[i];
    const op = NORMALISE[c] ?? c;
    if (c === "\\" && i + 1 < chars.length) {
      term += c + chars[++i];
    } else if (op === "&" || op === "|" || op === "!") {
      flush();
      tokens.push({ kind: "operator", text: c });
    } else if (op === "(" || op === ")") {
      flush();
      tokens.push({ kind: "paren", text: c });
    } else if (op === "~") {
      flush();
      let text = c;
      while (i + 1 < chars.length && /[\s\d]/.test(chars[i + 1])) text += chars[++i];
      // 後面的空白不算在 ~ 裡
      const trimmed = text.trimEnd();
      tokens.push({ kind: "near", text: trimmed });
      term = text.slice(trimmed.length);
    } else {
      term += c;
    }
  }
  flush();
  return tokens;
}
