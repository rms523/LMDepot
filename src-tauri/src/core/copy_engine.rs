use crate::error::{AppError, AppResult};
use crate::types::ModelFileRecord;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

const CHUNK_SIZE: usize = 10 * 1024 * 1024; // 10 MB read/write chunks

pub struct CopyProgress {
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current_file: String,
}

pub fn compute_total_bytes(files: &[ModelFileRecord]) -> u64 {
    files.iter().map(|f| f.size).sum()
}

pub fn copy_file_with_progress<F>(
    src: &Path,
    dst: &Path,
    mut on_progress: Option<F>,
) -> AppResult<u64>
where
    F: FnMut(u64),
{
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    let src_meta = fs::metadata(src)?;
    let src_size = src_meta.len();

    if dst.exists() {
        let dst_meta = fs::metadata(dst)?;
        if dst_meta.len() == src_size {
            if let (Ok(sm), Ok(dm)) = (src_meta.modified(), dst_meta.modified()) {
                if sm <= dm {
                    if let Some(ref mut cb) = on_progress {
                        cb(src_size);
                    }
                    return Ok(src_size);
                }
            }
        }
    }

    let mut src_file = File::open(src)?;
    let mut dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)?;

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut copied = 0u64;
    loop {
        let n = src_file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        dst_file.write_all(&buffer[..n])?;
        copied += n as u64;
        if let Some(ref mut cb) = on_progress {
            cb(copied);
        }
    }
    dst_file.sync_all()?;
    Ok(copied)
}

pub fn copy_model_files(
    source_root: &Path,
    dest_root: &Path,
    files: &[ModelFileRecord],
    mut progress: impl FnMut(CopyProgress),
) -> AppResult<u64> {
    let total = compute_total_bytes(files);
    let mut bytes_done = 0u64;

    for file in files {
        let src = source_root.join(&file.relative_path);
        let dst = dest_root.join(&file.relative_path);

        if !src.exists() {
            return Err(AppError::msg(format!(
                "Source file missing: {}",
                src.display()
            )));
        }

        let file_start = bytes_done;
        let current_file = file.relative_path.clone();

        progress(CopyProgress {
            bytes_done,
            bytes_total: total,
            current_file: current_file.clone(),
        });

        let copied = copy_file_with_progress(&src, &dst, Some(|chunk| {
            progress(CopyProgress {
                bytes_done: file_start + chunk,
                bytes_total: total,
                current_file: current_file.clone(),
            });
        }))?;
        bytes_done += copied;

        progress(CopyProgress {
            bytes_done,
            bytes_total: total,
            current_file,
        });
    }

    Ok(bytes_done)
}

pub fn hash_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK_SIZE];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn remove_dir_all(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub fn remove_file_or_dir(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(unix)]
pub fn create_symlink(target: &Path, link: &Path) -> AppResult<()> {
    use std::os::unix::fs::symlink;
    if link.exists() || link.symlink_metadata().is_ok() {
        remove_file_or_dir(link)?;
    }
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    symlink(target, link)?;
    Ok(())
}

#[cfg(windows)]
pub fn create_symlink(target: &Path, link: &Path) -> AppResult<()> {
    use std::os::windows::fs::symlink_dir;
    if link.exists() || link.symlink_metadata().is_ok() {
        remove_file_or_dir(link)?;
    }
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    symlink_dir(target, link)?;
    Ok(())
}

pub fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn move_dir(src: &Path, dst: &Path) -> AppResult<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    if dst.exists() {
        return Err(AppError::msg(format!(
            "Destination already exists: {}",
            dst.display()
        )));
    }
    fs::rename(src, dst)?;
    Ok(())
}
