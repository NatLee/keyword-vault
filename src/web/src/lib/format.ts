import type { Role, Visibility } from "../types";

export function dateTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString("zh-TW", { dateStyle: "medium", timeStyle: "short", hour12: false });
}

/** 「3 天前」這種寫法；超過一個月就給日期。 */
export function timeAgo(iso: string, now = Date.now()): string {
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return iso;
  const minutes = Math.floor((now - t) / 60_000);
  if (minutes < 1) return "剛剛";
  if (minutes < 60) return `${minutes} 分鐘前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小時前`;
  const days = Math.floor(hours / 24);
  if (days < 31) return `${days} 天前`;
  return new Date(iso).toLocaleDateString("zh-TW", { dateStyle: "medium" });
}

export const VISIBILITY_LABEL: Record<Visibility, string> = {
  private: "私人",
  group: "群組",
  public: "公開",
};

export const VISIBILITY_HINT: Record<Visibility, string> = {
  private: "只有你看得到",
  group: "和你同一個群組的人看得到",
  public: "所有登入的人都看得到",
};

export const ROLE_LABEL: Record<Role, string> = { admin: "管理員", member: "成員" };

/** 把「飲料, #熱門　茶」這種輸入拆成標籤。 */
export function parseTags(input: string): string[] {
  const tags: string[] = [];
  for (const raw of input.split(/[,，、;；\s]+/)) {
    const tag = raw.replace(/^#+/, "").trim();
    if (tag && !tags.includes(tag)) tags.push(tag);
  }
  return tags;
}
