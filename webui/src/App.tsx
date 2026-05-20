import { Routes, Route, Link, Navigate, useLocation } from "react-router-dom";
import AppProviders from "./AppProviders";
import BrowsePage from "./routes/BrowsePage";
import EntityPage from "./routes/EntityPage";
import GraphPage from "./routes/GraphPage";
import SearchPage from "./routes/SearchPage";
import PendingPanel from "./components/PendingPanel";

export default function App() {
  return (
    <AppProviders>
      <Shell />
    </AppProviders>
  );
}

function Shell() {
  return (
    <div style={{ display: "grid", gridTemplateRows: "48px 1fr", height: "100vh" }}>
      <header style={{
        background: "#1f2937", color: "#f9fafb",
        display: "flex", alignItems: "center", gap: 20, padding: "0 16px",
      }}>
        <strong>kg editor</strong>
        <NavLink to="/browse">Browse</NavLink>
        <NavLink to="/graph">Graph</NavLink>
        <NavLink to="/search">Search</NavLink>
      </header>
      <main style={{ overflow: "auto", paddingBottom: 200 /* room for PendingPanel */ }}>
        <Routes>
          <Route path="/" element={<Navigate to="/browse" replace />} />
          <Route path="/browse" element={<BrowsePage />} />
          <Route path="/entity/:id" element={<EntityPage />} />
          <Route path="/graph" element={<GraphPage />} />
          <Route path="/search" element={<SearchPage />} />
        </Routes>
      </main>
      <PendingPanel />
    </div>
  );
}

function NavLink({ to, children }: { to: string; children: React.ReactNode }) {
  const { pathname } = useLocation();
  const active = pathname.startsWith(to);
  return (
    <Link to={to} style={{
      color: "#f9fafb", textDecoration: "none",
      borderBottom: active ? "2px solid #f9fafb" : "2px solid transparent",
      padding: "12px 0",
    }}>
      {children}
    </Link>
  );
}
