import {
  BookOpen,
  GitCompareArrows,
  KeyRound,
  LayoutDashboard,
  LogOut,
  Menu,
  Monitor,
  Moon,
  Settings,
  Sun,
  X,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router";
import { useApp, useUser } from "../context";
import { ROLE_LABEL } from "../lib/format";
import { applyTheme, storedTheme, type ThemeChoice } from "../lib/theme";

interface NavItem {
  to: string;
  label: string;
  icon: LucideIcon;
  end?: boolean;
}

const NAV: NavItem[] = [
  { to: "/", label: "總覽", icon: LayoutDashboard, end: true },
  { to: "/keywords", label: "關鍵字", icon: KeyRound },
  { to: "/compare", label: "語法比對", icon: GitCompareArrows },
  { to: "/guide", label: "語法說明", icon: BookOpen },
  { to: "/settings", label: "設定", icon: Settings },
];

function Brand() {
  return (
    <Link to="/" className="flex items-center gap-2.5">
      <img src="/favicon.svg" alt="" className="size-8" />
      <span className="leading-tight">
        <span className="block text-sm font-semibold tracking-tight">Keyword Vault</span>
        <span className="block text-[11px] text-zinc-500 dark:text-zinc-400">關鍵字庫</span>
      </span>
    </Link>
  );
}

function navClass(isActive: boolean): string {
  return `flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition-colors ${
    isActive
      ? "bg-brand-50 font-medium text-brand-700 dark:bg-brand-950/70 dark:text-brand-200"
      : "text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
  }`;
}

function SidebarNav({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav className="flex flex-col gap-0.5">
      {NAV.map(({ to, label, icon: Icon, end }) => (
        <NavLink key={to} to={to} end={end} onClick={onNavigate} className={({ isActive }) => navClass(isActive)}>
          <Icon className="size-4" />
          {label}
        </NavLink>
      ))}
    </nav>
  );
}

const THEMES: { value: ThemeChoice; label: string; icon: LucideIcon }[] = [
  { value: "light", label: "淺色", icon: Sun },
  { value: "dark", label: "深色", icon: Moon },
  { value: "system", label: "跟系統", icon: Monitor },
];

function ThemeToggle() {
  const [theme, setTheme] = useState<ThemeChoice>(storedTheme);
  useEffect(() => applyTheme(theme), [theme]);
  return (
    <div className="flex rounded-lg bg-zinc-100 p-0.5 dark:bg-zinc-800" role="radiogroup" aria-label="外觀">
      {THEMES.map(({ value, label, icon: Icon }) => (
        <button
          key={value}
          role="radio"
          aria-checked={theme === value}
          title={label}
          onClick={() => setTheme(value)}
          className={`flex flex-1 items-center justify-center rounded-md py-1 transition-colors ${
            theme === value
              ? "bg-white text-brand-700 shadow-sm dark:bg-zinc-950 dark:text-brand-300"
              : "text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-100"
          }`}
        >
          <Icon className="size-3.5" />
          <span className="sr-only">{label}</span>
        </button>
      ))}
    </div>
  );
}

function SidebarFooter() {
  const { logout, meta } = useApp();
  const user = useUser();
  return (
    <div className="flex flex-col gap-2 border-t border-zinc-200 pt-3 dark:border-zinc-800">
      <div className="flex items-center gap-1">
        <span className="flex min-w-0 flex-1 items-center gap-2 px-2 py-1.5">
          <span className="grid size-7 shrink-0 place-items-center rounded-full bg-brand-100 text-xs font-semibold text-brand-700 dark:bg-brand-950 dark:text-brand-300">
            {Array.from(user.display_name)[0]}
          </span>
          <span className="min-w-0">
            <span className="block truncate text-sm font-medium">{user.display_name}</span>
            <span className="block text-[11px] text-zinc-500 dark:text-zinc-400">
              {ROLE_LABEL[user.role]}
              {meta ? `・v${meta.version}` : ""}
            </span>
          </span>
        </span>
        <button className="btn btn-ghost px-2" onClick={() => void logout()} title="登出">
          <LogOut className="size-4" />
          <span className="text-xs">登出</span>
        </button>
      </div>
      <ThemeToggle />
    </div>
  );
}

export function Layout() {
  const [open, setOpen] = useState(false);
  const location = useLocation();

  useEffect(() => setOpen(false), [location.pathname]);

  return (
    <div className="min-h-screen lg:flex">
      <aside className="sticky top-0 hidden h-screen w-60 shrink-0 flex-col gap-6 border-r border-zinc-200 bg-white px-3 py-4 lg:flex dark:border-zinc-800 dark:bg-zinc-900">
        <div className="px-2">
          <Brand />
        </div>
        <div className="flex-1 overflow-y-auto">
          <SidebarNav />
        </div>
        <SidebarFooter />
      </aside>

      <header className="sticky top-0 z-30 flex items-center justify-between border-b border-zinc-200 bg-white/90 px-4 py-2.5 backdrop-blur lg:hidden dark:border-zinc-800 dark:bg-zinc-900/90">
        <Brand />
        <button className="btn btn-ghost px-2" onClick={() => setOpen(true)} aria-label="開啟選單">
          <Menu className="size-5" />
        </button>
      </header>

      {open && (
        <div className="fixed inset-0 z-40 lg:hidden">
          <div className="absolute inset-0 bg-zinc-950/50" onClick={() => setOpen(false)} />
          <div className="absolute inset-y-0 left-0 flex w-64 flex-col gap-6 bg-white px-3 py-4 shadow-xl dark:bg-zinc-900">
            <div className="flex items-center justify-between px-2">
              <Brand />
              <button className="btn btn-ghost px-1.5" onClick={() => setOpen(false)} aria-label="關閉選單">
                <X className="size-4" />
              </button>
            </div>
            <div className="flex-1 overflow-y-auto">
              <SidebarNav onNavigate={() => setOpen(false)} />
            </div>
            <SidebarFooter />
          </div>
        </div>
      )}

      <main className="min-w-0 flex-1">
        <Outlet />
      </main>
    </div>
  );
}

export function PageBody({ children }: { children: ReactNode }) {
  return <div className="mx-auto max-w-6xl px-4 py-6 sm:px-6 lg:px-8 lg:py-8">{children}</div>;
}
