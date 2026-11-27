import { useEffect } from "react";
import { authorizeNext } from "./lib/access";
import { Navigate, Route, Routes, useLocation } from "react-router";
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
import { Access } from "./pages/Access";
import { Consent } from "./pages/Consent";
import "./access.css";
import { Inspector } from "./pages/Inspector";

function ContinueAuthorization({to}:{to:string}) { useEffect(()=>{window.location.replace(to)},[to]);return <p role="status" className="p-8">正在返回授權流程…</p>; }

export function App() {
  const location=useLocation();
  const next=location.pathname==="/login"?authorizeNext(location.search):null;
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
  if(next) return <ContinueAuthorization to={next}/>;
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<Dashboard />} />
        <Route path="keywords" element={<Keywords />} />
        <Route path="keywords/:id" element={<KeywordDetailPage />} />
        <Route path="compare" element={<Compare />} />
        <Route path="inspect" element={<Inspector />} />
        <Route path="guide" element={<Guide />} />
        <Route path="connections" element={<Access />} />
        <Route path="consent" element={<Consent />} />
        <Route path="settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
