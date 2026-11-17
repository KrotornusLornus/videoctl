use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("HOME no está definido"))
}

fn xdg_dir(var: &str, fallback: &str) -> Result<PathBuf> {
    match env::var_os(var) {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => Ok(home_dir()?.join(fallback)),
    }
}

pub fn data_dir() -> Result<PathBuf> {
    Ok(xdg_dir("XDG_DATA_HOME", ".local/share")?.join("videoctl"))
}

pub fn cache_dir() -> Result<PathBuf> {
    Ok(xdg_dir("XDG_CACHE_HOME", ".cache")?.join("videoctl"))
}

pub fn db_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("videos.db"))
}

pub fn thumbs_dir() -> Result<PathBuf> {
    Ok(cache_dir()?.join("thumbs"))
}

pub fn placeholder_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("empty.jpg"))
}

pub fn default_videos_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join("Videos"))
}

pub fn ensure_dirs() -> Result<()> {
    for dir in [data_dir()?, cache_dir()?, thumbs_dir()?] {
        fs::create_dir_all(&dir).with_context(|| format!("no se pudo crear {}", dir.display()))?;
    }
    Ok(())
}

pub fn expand_tilde(value: &str) -> Result<PathBuf> {
    if value == "~" {
        return home_dir();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return Ok(home_dir()?.join(rest));
    }
    Ok(PathBuf::from(value))
}

pub fn canonical_dir(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path)
        .with_context(|| format!("no se pudo resolver la carpeta {}", path.display()))
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn is_video_extension(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp4"
            | "mkv"
            | "webm"
            | "avi"
            | "mov"
            | "m4v"
            | "ts"
            | "m2ts"
            | "mts"
            | "flv"
            | "wmv"
            | "mpg"
            | "mpeg"
            | "ogv"
            | "3gp"
            | "mxf"
            | "vob"
            | "asf"
    )
}
