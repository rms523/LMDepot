import { useEffect, useState } from "react";
import { getDashboardStats, formatBytes } from "../api/client";
import type { DashboardStats } from "../types";

export function Dashboard() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getDashboardStats()
      .then(setStats)
      .catch((e) => setError(String(e)));
  }, []);

  if (error) return <div className="error">{error}</div>;
  if (!stats) return <div className="loading">Loading dashboard...</div>;

  return (
    <div className="dashboard">
      <h2>Dashboard</h2>
      <div className="stat-grid">
        <div className="stat-card">
          <div className="stat-value">{stats.total_models}</div>
          <div className="stat-label">Models discovered</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{formatBytes(stats.total_bytes)}</div>
          <div className="stat-label">Total storage</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{stats.backup_coverage_pct.toFixed(0)}%</div>
          <div className="stat-label">Backup coverage</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">
            {stats.mounted_drives}/{stats.drive_count}
          </div>
          <div className="stat-label">Drives mounted</div>
        </div>
      </div>
      <div className="stat-row">
        <div className="stat-inline">
          <strong>LM Studio:</strong> {formatBytes(stats.lmstudio_bytes)}
        </div>
        <div className="stat-inline">
          <strong>Unsloth (HF cache):</strong> {formatBytes(stats.unsloth_bytes)}
        </div>
      </div>
    </div>
  );
}
