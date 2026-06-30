# Model Backup

Cross-platform desktop app for backing up AI models from **LM Studio** and **Unsloth** (Hugging Face Hub cache) to one or more external drives.

## Features

- **Discover models** from LM Studio (`~/.lmstudio`, `~/.cache/lm-studio`, or `.lmstudio-home-pointer`) and Unsloth via HF cache (`~/.cache/huggingface/hub`)
- **Register multiple backup drives** (external SSDs, NAS mounts, etc.)
- **Backup** — full copy to a selected drive with `model.manifest.json`
- **Sync** — copy only missing or changed files (size + mtime comparison)
- **Backup all / Sync all** — batch backup or sync every discovered model to a chosen drive (one job, sequential)
- **Restore** — copy from backup back to the original or a custom path
- **Delete** — remove from source only, backup only, or both (with confirmation)
- **Offload** — move model to external drive and leave a symlink/junction at the original path so apps keep working
- **Job progress** — background operations with live progress in the Jobs tab

## Stack

- **Tauri 2** + **Rust** backend
- **React + TypeScript** frontend
- **SQLite** for model inventory, drives, and job history

## Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- Platform deps for Tauri: https://tauri.app/start/prerequisites/

## Development

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

Produces platform installers under `src-tauri/target/release/bundle/`.

## Tests

```bash
cd src-tauri && cargo test
```

## Default paths

| Source | Default location |
|--------|------------------|
| LM Studio | `~/.lmstudio/models` or `~/.cache/lm-studio/models` (see `~/.lmstudio-home-pointer`) |
| Unsloth / HF | `~/.cache/huggingface/hub` (or `HF_HUB_CACHE` / `HF_HOME`) |

Override paths in **Settings** if you relocated caches.

## Bulk backup / sync

On the **Models** page, pick a target drive from the dropdown and use **Backup all** or **Sync all** to process every discovered model in one background job.

On **Backup Drives**, each registered drive has its own **Backup all** / **Sync all** buttons. Progress appears in the **Jobs** tab.

## Backup layout on external drives

```
/Volumes/MyDrive/ModelBackup/
  lmstudio/
    author/Model-Name/
      model.manifest.json
      ...
  hf/
    unsloth/Model-Name/
      model.manifest.json
      ...
```

## Safety notes

- Close **LM Studio** and **Unsloth** before delete/offload operations (configurable in Settings).
- HF cache backups copy whole snapshot directories as real files — do not manually edit HF blob stores.
- On Windows, offload uses directory junctions; enable Developer Mode or run as admin if junction creation fails.
- Unplugging a drive during a job fails cleanly; re-run sync after remounting.

## Roadmap (v2)

- OMLX, Ollama, standalone Hugging Face source
- Custom folder watches
- Auto-sync on drive mount
- Optional cloud export via rclone/restic

## License

MIT
