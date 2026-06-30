import { NavLink, Route, Routes } from "react-router-dom";
import { useEffect } from "react";
import { startJobProgressListener } from "./jobProgress";
import { Dashboard } from "./pages/Dashboard";
import { DrivesPage } from "./pages/Drives";
import { JobsPage } from "./pages/Jobs";
import { ModelsPage } from "./pages/Models";
import { SettingsPage } from "./pages/Settings";
import "./App.css";

function App() {
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    startJobProgressListener().then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <h1>LMDepot</h1>
          <p>LM Studio · Hugging Face</p>
        </div>
        <nav>
          <NavLink to="/" end>
            Dashboard
          </NavLink>
          <NavLink to="/models">Models</NavLink>
          <NavLink to="/drives">Backup Drives</NavLink>
          <NavLink to="/jobs">Jobs</NavLink>
          <NavLink to="/settings">Settings</NavLink>
        </nav>
      </aside>
      <main className="content">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/models" element={<ModelsPage />} />
          <Route path="/drives" element={<DrivesPage />} />
          <Route path="/jobs" element={<JobsPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
}

export default App;
