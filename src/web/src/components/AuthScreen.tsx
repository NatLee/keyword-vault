import type { ReactNode } from "react";
import { RegistrationLoginLink, RegistrationNotice } from "../registration/RegisterForm";
import { LogIn, UserPlus } from "lucide-react";
import { useRef, useState } from "react";
import { Providers } from "../components/Providers";
import { ErrorBanner, Spinner } from "../components/ui";
import { ThemeToggle } from "../components/Layout";
import { useApp } from "../context";
import { errorText } from "../lib/hooks";

/** Shared themed account surface; lifecycle is owned by the route pages. */
export function AuthScreen({ setupRequired, children }: { setupRequired: boolean; children?: ReactNode }) {
  const { login, setup, meta, auth } = useApp();
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [setupToken, setSetupToken] = useState("");
  const tokenRequired = setupRequired && Boolean(auth?.setup_token_required);
  const submitting = useRef(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    if (submitting.current) return;
    setError(null);
    if (setupRequired) {
      if (!/^[a-zA-Z0-9._-]{3,32}$/.test(username.trim())) { setError("帳號要 3 到 32 個字元，只能用英數字和 _ . -"); return; }
      if ([...password].length < 8 || [...password].length > 200) { setError("密碼需為 8 到 200 個字元"); return; }
      if ([...displayName.trim()].length > 40) { setError("顯示名稱最多 40 個字元"); return; }
      if (password !== confirmation) { setError("兩次輸入的密碼不一致"); return; }
    }
    submitting.current = true;
    setBusy(true);
    try {
      if (setupRequired) {
        await setup({
          username: username.trim(),
          display_name: displayName.trim(),
          password,
          ...(tokenRequired ? {setup_token:setupToken} : {}),
        });
      } else {
        await login(username.trim(), password);
      }
      setPassword("");
      setConfirmation("");
      setSetupToken("");
    } catch (e) {
      setError(errorText(e));
      if (!setupRequired) setPassword("");
    } finally {
      submitting.current = false;
      setBusy(false);
    }
  };

  return (
    <div className="login-workspace">
      <section className="login-intro">
        <div className="login-wordmark">
          <img src="/favicon.svg" alt="" /> KEYWORD VAULT
        </div>
        <div>
          <p className="eyebrow">THE QUERY CABINET / 關鍵字典藏室</p>
          <h2>
            把好語法，
            <br />
            留給<em>下一次靈感。</em>
          </h2>
          <p className="login-description">
            一間收藏搜尋知識的房間。編入主題、拆開推敲，讓團隊的每一次發現，都能被再次找到。
          </p>
        </div>
        <div className="login-specimen">
          <span>INDEX CARD / 001</span>
          <code>
            (珍珠奶茶 <b>|</b> 波霸奶茶)
            <br />
            <b>&amp;</b> (推薦 <b>|</b> 評價) <b>!</b> 業配
          </code>
          <div>
            <span>01 收集</span>
            <span>02 驗證</span>
            <span>03 分享</span>
          </div>
        </div>
      </section>
      <section className="login-access">
        <div className="login-theme">
          <ThemeToggle />
        </div>
        {children ?? <form
          className="login-form w-full max-w-sm"
          aria-busy={busy}
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <p className="eyebrow mb-3">READER’S ACCESS / 閱覽證</p>
          <div className="mb-8 flex items-center gap-3">
            <img src="/favicon.svg" alt="" className="size-10" />
            <div>
              <h1 className="text-3xl font-semibold tracking-tight">
                {setupRequired ? "建立第一位管理員" : "歡迎回來"}
              </h1>
              <p className="text-xs text-zinc-500 dark:text-zinc-400">
                {setupRequired
                  ? "首次啟用 / FIRST LOGIN"
                  : "開啟你的關鍵字典藏室"}
                {meta ? `・v${meta.version}` : ""}
              </p>
            </div>
          </div>
          {setupRequired && <div className="first-login-note">
            <p className="eyebrow">01 建立帳號 / 02 邀請團隊</p>
            <p>由你保管工作空間的第一把鑰匙。建立後即可登入，再由管理員設定成員註冊與審核。</p>
          </div>}
          <label className="label" htmlFor="login-username">
            帳號
          </label>
          <input
            id="login-username"
            className="input"
            autoComplete="username"
            autoFocus
            value={username}
            onChange={(e) => setUsername(e.target.value)}
          />
          {setupRequired && (
            <>
              <p className="hint">3 到 32 個字元：英文字母、數字、. _ -</p>
              <label className="label mt-3" htmlFor="login-display-name">
                顯示名稱
              </label>
              <input
                id="login-display-name"
                className="input"
                placeholder="其他人看到的名字，可以不填"
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
              />
            </>
          )}
          <label className="label mt-3" htmlFor="login-password">
            密碼
          </label>
          <input
            id="login-password"
            className="input"
            type="password"
            autoComplete={setupRequired ? "new-password" : "current-password"}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          {setupRequired && <>
            <p className="hint">8 到 200 個字元。</p>
            <label className="label mt-3" htmlFor="login-confirmation">確認密碼</label>
            <input id="login-confirmation" className="input" type="password" autoComplete="new-password" value={confirmation} onChange={e=>setConfirmation(e.target.value)} />
          </>}
          {tokenRequired && <>
            <label className="label mt-3" htmlFor="deployment-code">部署設定代碼</label>
            <input id="deployment-code" className="input" type="password" autoComplete="off" value={setupToken} onChange={e=>setSetupToken(e.target.value)} />
            <p className="hint">向部署管理員取得設定代碼。代碼只用於建立第一位管理員。</p>
            {!auth?.setup_token_configured && <p role="alert" className="hint">部署管理員尚未設定代碼，請聯絡部署管理員完成設定。</p>}
          </>}
          {error && (
            <div className="mt-4">
              <ErrorBanner message={error} />
            </div>
          )}
          <button
            className="btn btn-primary mt-5 w-full py-2"
            type="submit"
            disabled={busy || !username.trim() || !password || (setupRequired && !confirmation) || (tokenRequired && (!setupToken.trim() || !auth?.setup_token_configured))}
          >
            {busy ? (
              <Spinner />
            ) : setupRequired ? (
              <UserPlus className="size-4" />
            ) : (
              <LogIn className="size-4" />
            )}
            {setupRequired ? "建立管理員" : "登入"}
          </button>
          <p className="mt-4 text-xs text-zinc-500 dark:text-zinc-400">
            {setupRequired
              ? "這個帳號可以新增其他成員和群組。之後這個畫面就不會再出現。"
              : "沒有帳號或忘記密碼，請找管理員。"}
          </p>
          {!setupRequired && <><RegistrationNotice /><RegistrationLoginLink /><Providers /></>}
        </form>}
        <p className="login-footnote">寫好一次，讓好語法持續累積。</p>
      </section>
    </div>
  );
}
