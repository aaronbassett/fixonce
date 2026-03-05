import { BrowserRouter, Routes, Route, NavLink } from "react-router-dom";
import { Dashboard } from "./views/Dashboard.tsx";
import { MemoryQuery } from "./views/MemoryQuery.tsx";
import { MemoryDetail } from "./views/MemoryDetail.tsx";
import { CreateMemory } from "./views/CreateMemory.tsx";
import { RecentFeedback } from "./views/RecentFeedback.tsx";
import { RecentActivity } from "./views/RecentActivity.tsx";

export function App() {
  return (
    <BrowserRouter>
      <div style={{ maxWidth: "960px", margin: "0 auto", padding: "1rem" }}>
        <nav style={navStyle}>
          <NavLink to="/" style={linkStyle} end>
            Dashboard
          </NavLink>
          <NavLink to="/search" style={linkStyle}>
            Search
          </NavLink>
          <NavLink to="/create" style={linkStyle}>
            Create
          </NavLink>
          <NavLink to="/feedback" style={linkStyle}>
            Feedback
          </NavLink>
          <NavLink to="/activity" style={linkStyle}>
            Activity
          </NavLink>
        </nav>

        <main style={{ marginTop: "1rem" }}>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/search" element={<MemoryQuery />} />
            <Route path="/create" element={<CreateMemory />} />
            <Route path="/memories/:id" element={<MemoryDetail />} />
            <Route path="/feedback" element={<RecentFeedback />} />
            <Route path="/activity" element={<RecentActivity />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}

const navStyle: React.CSSProperties = {
  display: "flex",
  gap: "1rem",
  padding: "0.75rem 0",
  borderBottom: "1px solid #ddd",
};

const linkStyle: React.CSSProperties = {
  textDecoration: "none",
  color: "#333",
  padding: "0.25rem 0.5rem",
  borderRadius: "3px",
  fontSize: "0.9rem",
};
