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
  { to: "/", label: "工作台", icon: LayoutDashboard, end: true },
  { to: "/keywords", label: "語法庫", icon: KeyRound },
  { to: "/inspect", label: "語法解剖", icon: GitCompareArrows },
  { to: "/compare", label: "語法比對", icon: GitCompareArrows },
  { to: "/guide", label: "語法說明", icon: BookOpen },
  { to: "/settings", label: "設定", icon: Settings },
];

function Brand() {
  return (
    <Link to="/" className="vault-brand flex items-center gap-3">
      <img src="/favicon.svg" alt="" className="size-9" />
      <span className="leading-tight">
        <span className="block text-base font-semibold tracking-tight">
          Keyword Vault
        </span>
        <span className="block text-[11px] text-zinc-500 dark:text-zinc-400">
          THE QUERY CABINET
        </span>
      </span>
    </Link>
  );
}

function SidebarNav({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav className="vault-nav" aria-label="主要導覽">
      {NAV.map(({ to, label, icon: Icon, end }, index) => (
        <NavLink
          key={to}
          to={to}
          end={end}
          onClick={onNavigate}
          className={({ isActive }) =>
            `vault-nav-link ${isActive ? "active" : ""}`
          }
        >
          <span className="nav-index">0{index + 1}</span>
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

export function ThemeToggle() {
  const [theme, setTheme] = useState<ThemeChoice>(storedTheme);
  useEffect(() => applyTheme(theme), [theme]);
  return (
    <div
      className="flex rounded-lg bg-zinc-100 p-0.5 dark:bg-zinc-800"
      role="radiogroup"
      aria-label="外觀"
    >
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

export function Layout() {
  const [open, setOpen] = useState(false);
  const location = useLocation();
  const { logout, meta } = useApp();
  const user = useUser();
  useEffect(() => setOpen(false), [location.pathname]);
  useEffect(() => {
    if (!open) return;
    const close = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [open]);
  return (
    <div className="vault-shell min-h-screen">
      <a href="#workspace" className="skip-link">
        跳到工作區
      </a>
      <header className="vault-masthead">
        <div className="vault-masthead-inner">
          <Brand />
          <span className="vault-edition hidden md:block">
            ARCHIVE EDITION / v{meta?.version ?? "1.0"}
          </span>
          <div className="ml-auto flex items-center gap-3">
            <span className="hidden text-right text-xs sm:block">
              <strong className="block font-medium">{user.display_name}</strong>
              <span className="text-zinc-500">{ROLE_LABEL[user.role]}</span>
            </span>
            <button
              className="btn btn-ghost px-2"
              onClick={() => void logout()}
              aria-label="登出"
            >
              <LogOut className="size-4" />
            </button>
            <button
              className="btn btn-ghost px-2 md:hidden"
              onClick={() => setOpen(!open)}
              aria-label={open ? "關閉選單" : "開啟選單"}
              aria-expanded={open}
            >
              {open ? <X className="size-5" /> : <Menu className="size-5" />}
            </button>
          </div>
        </div>
        <div className={`vault-navigation ${open ? "is-open" : ""}`}>
          <SidebarNav onNavigate={() => setOpen(false)} />
          <div className="navigation-theme">
            <ThemeToggle />
          </div>
          <Link className="vault-syntax-link" to="/guide">
            <code>&amp; AND</code>
            <code>| OR</code>
            <code>! NOT</code>
            <code>~ NEAR</code>
          </Link>
        </div>
      </header>
      <main id="workspace" className="min-w-0" tabIndex={-1}>
        <Outlet />
      </main>
      <footer className="vault-footer">
        <span>KEYWORD VAULT / 好語法，值得典藏。</span>
        <Link to="/guide">語法參考 →</Link>
      </footer>
    </div>
  );
}

export function PageBody({ children }: { children: ReactNode }) {
  return (
    <div className="workspace-body mx-auto max-w-7xl px-4 py-6 sm:px-8 lg:px-10 lg:py-9">
      {children}
    </div>
  );
}
