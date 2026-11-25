import { Navigate, Route, Routes } from "react-router";
import { Layout } from "./components/Layout";
import { ErrorBanner, Loading } from "./components/ui";
import { useApp } from "./context";
import { Compare } from "./pages/Compare";
import { Dashboard } from "./pages/Dashboard";
import { Guide } from "./pages/Guide";
import { KeywordDetailPage } from "./pages/KeywordDetail";
import { Keywords } from "./pages/Keywords";
import { Login } from "./pages/Login";
import { Settings } from "./pages/Settings";

export function App() {
  const { auth, authError } = useApp();
  if (!auth) {
    return (
      <div className="mx-auto max-w-md px-4 py-10">
        {authError ? <ErrorBanner message={`連不上伺服器：${authError}`} /> : <Loading />}
      </div>
    );
  }
  // 還沒登入（或還沒有任何帳號）：只有登入頁
  if (!auth.user) return <Login setupRequired={auth.setup_required} />;
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<Dashboard />} />
        <Route path="keywords" element={<Keywords />} />
        <Route path="keywords/:id" element={<KeywordDetailPage />} />
        <Route path="compare" element={<Compare />} />
        <Route path="guide" element={<Guide />} />
        <Route path="settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
