use crate::db::{self, Video};
use crate::paths;
use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
pub enum ThumbMethod {
    Auto,
    Ffmpeg,
    Ytdlp,
}

impl ThumbMethod {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "ffmpeg" => Ok(Self::Ffmpeg),
            "ytdlp" | "yt-dlp" => Ok(Self::Ytdlp),
            _ => Err(anyhow!(
                "método inválido: {value} (usa auto, ffmpeg o ytdlp)"
            )),
        }
    }
}

pub fn thumb_path_for(id: i64) -> Result<PathBuf> {
    Ok(paths::thumbs_dir()?.join(format!("{id}.jpg")))
}

pub fn fallback_path_for(id: i64) -> Result<PathBuf> {
    Ok(paths::thumbs_dir()?
        .join("missing")
        .join(format!("{id}.jpg")))
}

pub fn ensure_fallback(id: i64) -> Result<PathBuf> {
    let path = fallback_path_for(id)?;
    if path.is_file() {
        return Ok(path);
    }
    copy_placeholder(&path)?;
    Ok(path)
}

pub fn remove_fallback(id: i64) {
    if let Ok(path) = fallback_path_for(id) {
        let _ = fs::remove_file(path);
    }
}

fn temp_path(id: i64) -> Result<PathBuf> {
    let dir = paths::thumbs_dir()?;
    fs::create_dir_all(&dir)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(dir.join(format!(".{id}-{}-{stamp}.tmp.jpg", std::process::id())))
}

fn stderr_tail(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().chars().take(300).collect())
        .unwrap_or_else(|| "sin detalle".to_string())
}

pub fn ensure_placeholder() -> Result<PathBuf> {
    let path = paths::placeholder_path()?;
    if path.exists() {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x202020:s=480x270",
            "-frames:v",
            "1",
            "-q:v",
            "5",
        ])
        .arg(&path)
        .status()
        .context("no se pudo ejecutar ffmpeg")?;
    if !status.success() {
        return Err(anyhow!("no se pudo crear la imagen vacía"));
    }
    Ok(path)
}

fn copy_placeholder(target: &Path) -> Result<()> {
    let placeholder = ensure_placeholder()?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&placeholder, target)?;
    Ok(())
}

fn generate_frame(path: &Path, out: &Path, duration: Option<f64>) -> Result<()> {
    let seek = duration
        .filter(|d| *d > 0.0)
        .map(|d| (d * 0.15).clamp(1.0, 120.0))
        .unwrap_or(0.0);
    let attempts = if seek > 0.0 {
        vec![seek, 0.0]
    } else {
        vec![0.0]
    };
    for position in attempts {
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-y",
                "-ss",
                &format!("{position:.3}"),
                "-i",
            ])
            .arg(path)
            .args(["-frames:v", "1", "-vf", "scale=480:-2", "-q:v", "3"])
            .arg(out)
            .status()
            .context("no se pudo ejecutar ffmpeg")?;
        if status.success() && out.exists() {
            return Ok(());
        }
    }
    Err(anyhow!(
        "no se pudo extraer un fotograma de {}",
        path.display()
    ))
}

fn convert_to_thumb(source: &Path, out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"])
        .arg(source)
        .args(["-frames:v", "1", "-vf", "scale=480:-2", "-q:v", "3"])
        .arg(out)
        .status()
        .context("no se pudo ejecutar ffmpeg")?;
    if status.success() && out.exists() {
        Ok(())
    } else {
        Err(anyhow!("no se pudo convertir la imagen a thumbnail"))
    }
}

fn download_remote(url: &str, out: &Path) -> Result<()> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = paths::cache_dir()?.join(format!("remote-thumb-{}-{stamp}.tmp", std::process::id()));
    let status = Command::new("curl")
        .args(["-fsSL", "--max-time", "30", "-o"])
        .arg(&temp)
        .arg(url)
        .status()
        .context("no se pudo ejecutar curl")?;
    if !status.success() || !temp.exists() {
        let _ = fs::remove_file(&temp);
        return Err(anyhow!("no se pudo descargar el thumbnail remoto"));
    }
    let result = convert_to_thumb(&temp, out);
    let _ = fs::remove_file(&temp);
    result
}

fn find_ytdlp_thumbnail(dir: &Path) -> Result<PathBuf> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();
        if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
            return Ok(path);
        }
    }
    Err(anyhow!("yt-dlp no generó ningún thumbnail"))
}

fn download_ytdlp(url: &str, out: &Path) -> Result<()> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = paths::cache_dir()?.join(format!("ytdlp-thumb-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir)?;
    let result = (|| -> Result<()> {
        let template = dir.join("thumb.%(ext)s");
        let output = Command::new("yt-dlp")
            .args([
                "--no-playlist",
                "--skip-download",
                "--no-warnings",
                "--no-progress",
                "--write-thumbnail",
                "--convert-thumbnails",
                "jpg",
                "--output",
            ])
            .arg(&template)
            .arg("--")
            .arg(url)
            .output()
            .context("no se pudo ejecutar yt-dlp")?;
        if !output.status.success() {
            return Err(anyhow!("yt-dlp falló: {}", stderr_tail(&output.stderr)));
        }
        let source = find_ytdlp_thumbnail(&dir)?;
        convert_to_thumb(&source, out)
    })();
    let _ = fs::remove_dir_all(&dir);
    result
}

fn materialize_ffmpeg(video: &Video, out: &Path) -> Result<()> {
    let Some(local) = &video.path else {
        return Err(anyhow!("el vídeo no tiene fichero local; usa ytdlp"));
    };
    if !Path::new(local).exists() {
        return Err(anyhow!("el fichero local no existe"));
    }
    if !video.has_video {
        return Err(anyhow!("el fichero local no tiene pista de vídeo"));
    }
    generate_frame(Path::new(local), out, video.duration)
}

fn materialize_ytdlp(video: &Video, out: &Path) -> Result<()> {
    let Some(url) = &video.url else {
        return Err(anyhow!("el vídeo no tiene URL; usa ffmpeg"));
    };
    download_ytdlp(url, out)
}

fn materialize_auto(video: &Video, out: &Path) -> bool {
    if materialize_ffmpeg(video, out).is_ok() {
        return true;
    }
    if let Some(url) = &video.thumb_url {
        if download_remote(url, out).is_ok() {
            return true;
        }
    }
    if materialize_ytdlp(video, out).is_ok() {
        return true;
    }
    false
}

pub fn regenerate_with(conn: &Connection, video: &Video, method: ThumbMethod) -> Result<String> {
    let target = thumb_path_for(video.id)?;
    let temp = temp_path(video.id)?;
    let result = match method {
        ThumbMethod::Auto => {
            if materialize_auto(video, &temp) {
                Ok(())
            } else {
                copy_placeholder(&temp)
            }
        }
        ThumbMethod::Ffmpeg => materialize_ffmpeg(video, &temp),
        ThumbMethod::Ytdlp => materialize_ytdlp(video, &temp),
    };
    if let Err(err) = result {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&temp, &target).with_context(|| {
        format!(
            "no se pudo mover el thumbnail temporal a {}",
            target.display()
        )
    })?;
    remove_fallback(video.id);
    let thumb = target.to_string_lossy().to_string();
    db::set_thumb(conn, video.id, &thumb)?;
    Ok(thumb)
}

pub fn ensure_thumb_with(conn: &Connection, video: &Video, method: ThumbMethod) -> Result<String> {
    if let Some(thumb) = &video.thumb_path {
        if Path::new(thumb).exists() {
            remove_fallback(video.id);
            return Ok(thumb.clone());
        }
    }
    regenerate_with(conn, video, method)
}

pub fn ensure_thumb(conn: &Connection, video: &Video) -> Result<String> {
    ensure_thumb_with(conn, video, ThumbMethod::Auto)
}

pub fn regenerate_local(conn: &Connection, video: &Video) -> Result<String> {
    match regenerate_with(conn, video, ThumbMethod::Ffmpeg) {
        Ok(thumb) => Ok(thumb),
        Err(_) => {
            let target = thumb_path_for(video.id)?;
            let temp = temp_path(video.id)?;
            if let Err(err) = copy_placeholder(&temp) {
                let _ = fs::remove_file(&temp);
                return Err(err);
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&temp, &target)?;
            remove_fallback(video.id);
            let thumb = target.to_string_lossy().to_string();
            db::set_thumb(conn, video.id, &thumb)?;
            Ok(thumb)
        }
    }
}

pub fn ensure_many(conn: &Connection, videos: &[Video], method: ThumbMethod) -> String {
    let mut ok = 0usize;
    let mut errors = 0usize;
    for video in videos {
        match ensure_thumb_with(conn, video, method) {
            Ok(_) => ok += 1,
            Err(err) => {
                errors += 1;
                eprintln!("aviso: miniatura de {}: {err:#}", video.title);
            }
        }
    }
    format!("Miniaturas: {ok} listas, {errors} errores.")
}

pub fn regenerate_many(conn: &Connection, videos: &[Video], method: ThumbMethod) -> String {
    let mut ok = 0usize;
    let mut errors = 0usize;
    for video in videos {
        match regenerate_with(conn, video, method) {
            Ok(_) => ok += 1,
            Err(err) => {
                errors += 1;
                eprintln!("aviso: miniatura de {}: {err:#}", video.title);
            }
        }
    }
    format!("Miniaturas regeneradas: {ok}, errores: {errors}.")
}

pub fn remove_thumb(path: Option<&str>) {
    if let Some(path) = path {
        let path = Path::new(path);
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".jpg"))
            .unwrap_or(false)
        {
            let _ = fs::remove_file(path);
        }
    }
}
