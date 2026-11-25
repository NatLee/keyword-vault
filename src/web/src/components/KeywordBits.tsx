import { Combine, Globe, Lock, Star, Users } from "lucide-react";
import { Link } from "react-router";
import { VISIBILITY_HINT, VISIBILITY_LABEL } from "../lib/format";
import type { Keyword, Visibility } from "../types";
import { Badge, type BadgeTone } from "./ui";

const VISIBILITY_STYLE: Record<Visibility, { tone: BadgeTone; icon: typeof Lock }> = {
  private: { tone: "neutral", icon: Lock },
  group: { tone: "sky", icon: Users },
  public: { tone: "green", icon: Globe },
};

export function VisibilityBadge({ visibility }: { visibility: Visibility }) {
  const { tone, icon: Icon } = VISIBILITY_STYLE[visibility];
  return (
    <Badge tone={tone} title={VISIBILITY_HINT[visibility]}>
      <Icon className="size-3" />
      {VISIBILITY_LABEL[visibility]}
    </Badge>
  );
}

export function CombinationBadge() {
  return (
    <Badge tone="brand" title="用其他關鍵字組起來的">
      <Combine className="size-3" />
      組合
    </Badge>
  );
}

export function KeywordBadges({ keyword }: { keyword: Keyword }) {
  return (
    <>
      {keyword.is_combination && <CombinationBadge />}
      <VisibilityBadge visibility={keyword.visibility} />
    </>
  );
}

export function TagLink({ tag }: { tag: string }) {
  return (
    <Link
      to={`/keywords?tag=${encodeURIComponent(tag)}`}
      className="rounded-md bg-zinc-100 px-1.5 py-0.5 text-xs text-zinc-600 hover:bg-brand-100 hover:text-brand-800 dark:bg-zinc-800 dark:text-zinc-300 dark:hover:bg-brand-950 dark:hover:text-brand-200"
    >
      #{tag}
    </Link>
  );
}

export function FavoriteButton({
  favorite,
  onToggle,
  withLabel,
}: {
  favorite: boolean;
  onToggle: () => void;
  withLabel?: boolean;
}) {
  return (
    <button
      type="button"
      className={`btn ${withLabel ? "btn-secondary" : "btn-ghost px-1.5"}`}
      onClick={onToggle}
      aria-pressed={favorite}
      title={favorite ? "取消收藏" : "收藏"}
    >
      <Star className={`size-4 ${favorite ? "fill-amber-400 text-amber-500" : "text-zinc-400"}`} />
      {withLabel ? (favorite ? "已收藏" : "收藏") : <span className="sr-only">{favorite ? "取消收藏" : "收藏"}</span>}
    </button>
  );
}
