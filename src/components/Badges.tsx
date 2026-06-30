import { formatBytes } from "../api/client";

interface Props {
  pct: number;
  label?: string;
}

export function ProgressBar({ pct, label }: Props) {
  const clamped = Math.min(100, Math.max(0, pct));
  return (
    <div className="progress-wrap">
      {label && <div className="progress-label">{label}</div>}
      <div className="progress-track">
        <div className="progress-fill" style={{ width: `${clamped}%` }} />
      </div>
      <div className="progress-pct">{clamped.toFixed(1)}%</div>
    </div>
  );
}

export function SizeBadge({ bytes }: { bytes: number }) {
  return <span className="badge">{formatBytes(bytes)}</span>;
}

export function SourceBadge({ source }: { source: string }) {
  const label = source === "lmstudio" ? "LM Studio" : source === "unsloth" ? "Unsloth" : source;
  return <span className={`badge source-${source}`}>{label}</span>;
}

export function StatusBadge({ status }: { status: string }) {
  return <span className={`badge status-${status}`}>{status.replace("_", " ")}</span>;
}
