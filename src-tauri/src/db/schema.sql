CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    source TEXT NOT NULL,
    primary_path TEXT NOT NULL,
    total_bytes INTEGER NOT NULL,
    file_count INTEGER NOT NULL,
    scanned_at TEXT NOT NULL,
    revision TEXT
);

CREATE TABLE IF NOT EXISTS model_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model_id TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    FOREIGN KEY (model_id) REFERENCES models(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_model_files_model_id ON model_files(model_id);

CREATE TABLE IF NOT EXISTS backup_drives (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    root_path TEXT NOT NULL,
    volume_id TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    last_seen_at TEXT
);

CREATE TABLE IF NOT EXISTS model_backups (
    id TEXT PRIMARY KEY,
    model_id TEXT NOT NULL,
    drive_id TEXT NOT NULL,
    backup_path TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'backed_up',
    last_synced_at TEXT,
    FOREIGN KEY (model_id) REFERENCES models(id) ON DELETE CASCADE,
    FOREIGN KEY (drive_id) REFERENCES backup_drives(id) ON DELETE CASCADE,
    UNIQUE(model_id, drive_id)
);

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,
    status TEXT NOT NULL,
    model_id TEXT,
    drive_id TEXT,
    progress_pct REAL NOT NULL DEFAULT 0,
    bytes_done INTEGER NOT NULL DEFAULT 0,
    bytes_total INTEGER NOT NULL DEFAULT 0,
    current_file TEXT,
    message TEXT,
    created_at TEXT NOT NULL,
    finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at DESC);
