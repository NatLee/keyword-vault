import { Register } from "./pages/Register";
import { useEffect } from "react";
import { authorizeNext, providerError } from "./lib/access";
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
import { FirstLogin } from "./pages/FirstLogin";
import { Settings } from "./pages/Settings";
import { Access } from "./pages/Access";
import { Consent } from "./pages/Consent";
import "./access.css";
import { Inspector } from "./pages/Inspector";

function ContinueAuthorization({to}:{to:string}) { useEffect(()=>{window.location.replace(to)},[to]);return <p role="status" className="p-8">正在返回授權流程…</p>; }

export function App() {
  const location=useLocation();
  const next=authorizeNext(location.search);
  const { auth, authError } = useApp();
  if (!auth) {
    return (
      <div className="mx-auto max-w-md px-4 py-10">
        {authError ? <ErrorBanner message={`連不上伺服器：${authError}`} /> : <Loading />}
      </div>
    );
  }
  if (!auth.user && !auth.setup_required && location.pathname === "/register") return <Register />;
  if (!auth.user) {
    const path = auth.setup_required ? "/first-login" : "/login";
    const params = new URLSearchParams();
    if (next) params.set("next", next);
    if (!auth.setup_required && providerError(location.search)) {
      params.set("access_error", new URLSearchParams(location.search).get("access_error")!);
    }
    if (!auth.setup_required && new URLSearchParams(location.search).get("registration") === "pending") params.set("registration", "pending");
    const search = params.size ? `?${params}` : "";
    // Canonicalize entry and drop untrusted continuation parameters.
    if (location.pathname !== path || location.search !== search) return <Navigate to={path + search} replace />;
    return auth.setup_required ? <FirstLogin /> : <Login />;
  }
  if(next) return <ContinueAuthorization to={next}/>;
  if (["/login", "/first-login", "/register"].includes(location.pathname)) return <Navigate to="/" replace />;
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
