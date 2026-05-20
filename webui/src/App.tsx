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
      <div className="grid grid-rows-[56px_1fr] h-screen">
        <Header />
        <main className="overflow-hidden">
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
    </AppProviders>
  );
}

function Header() {
  return (
    <header className="flex items-center gap-6 px-6 bg-slate-900 text-slate-50 border-b border-slate-700">
      <Link to="/" className="font-bold tracking-tight text-base">kg editor</Link>
      <nav className="flex gap-1">
        <NavLink to="/browse">Browse</NavLink>
        <NavLink to="/graph">Graph</NavLink>
        <NavLink to="/search">Search</NavLink>
      </nav>
    </header>
  );
}

function NavLink({ to, children }: { to: string; children: React.ReactNode }) {
  const { pathname } = useLocation();
  const active = pathname.startsWith(to);
  return (
    <Link
      to={to}
      className={`px-3 py-1.5 rounded-md text-sm transition-colors ${
        active ? "bg-slate-800 text-white" : "text-slate-300 hover:bg-slate-800 hover:text-white"
      }`}
    >
      {children}
    </Link>
  );
}
