use crate::db::{self, Video};
use crate::fingerprint;
use crate::lookup;
use crate::paths;
use crate::probe;
use crate::thumbs;
use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const QUALITIES: &[(&str, &str)] = &[
    (
        "1080p (máximo)",
        "bestvideo[height<=1080]+bestaudio/best[height<=1080]",
    ),
    (
        "720p (máximo)",
        "bestvideo[height<=720]+bestaudio/best[height<=720]",
    ),
    (
        "480p (máximo)",
        "bestvideo[height<=480]+bestaudio/best[height<=480]",
    ),
    ("Mejor disponible", "bestvideo+bestaudio/best"),
    ("Solo audio", "bestaudio/best"),
];

fn notify(title: &str, body: &str, critical: bool) {
    let mut command = Command::new("fyi");
    command.args(["-a", "videoctl"]);
    if critical {
        command.args(["-u", "critical", "-t", "60000"]);
    }
    command
        .arg(title)
        .arg(body)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _ = command.status();
}

fn clean_error(value: &str) -> String {
    let text = value
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(value)
        .trim();
    let mut chars: String = text.chars().take(300).collect();
    if text.chars().count() > 300 {
        chars.push('…');
    }
    chars
}

fn active_download(conn: &Connection, video: &Video) -> bool {
    matches!(
        db::download_state(conn, video.id).ok().flatten(),
        Some((state, _)) if state == "queued" || state == "downloading"
    )
}

pub fn qualities() -> &'static [(&'static str, &'static str)] {
    QUALITIES
}

pub fn candidates(conn: &Connection, thumbs: &[String]) -> Result<Vec<Video>> {
    let videos = lookup::by_thumbs(conn, thumbs)?;
    let mut candidates = Vec::new();
    for video in videos {
        if video.url.is_none()
            || db::path_exists(video.path.as_ref())
            || active_download(conn, &video)
        {
            continue;
        }
        candidates.push(video);
    }
    Ok(candidates)
}

pub fn enqueue(conn: &Connection, thumbs: &[String], format: &str, dir: &Path) -> Result<String> {
    let candidates = candidates(conn, thumbs)?;
    if candidates.is_empty() {
        return Ok("Nada que descargar: ya son locales o no tienen URL.".to_string());
    }
    fs::create_dir_all(dir).with_context(|| format!("no se pudo crear {}", dir.display()))?;
    let dir_text = dir.to_string_lossy().to_string();
    for video in &candidates {
        db::set_download_state(
            conn,
            video.id,
            "queued",
            Some(format),
            Some(&dir_text),
            None,
            None,
        )?;
    }

    let executable = std::env::current_exe().context("no se pudo resolver el ejecutable")?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths::cache_dir()?.join("download.log"))
        .ok();
    let mut command = Command::new(executable);
    command.arg("download-worker");
    for video in &candidates {
        command.arg(video.id.to_string());
    }
    command
        .arg("--format")
        .arg(format)
        .arg("--dir")
        .arg(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    if let Some(log) = log {
        command.stderr(Stdio::from(log));
    } else {
        command.stderr(Stdio::null());
    }
    command
        .spawn()
        .context("no se pudo iniciar el descargador")?;

    let mut preview = dir_text;
    if preview.chars().count() > 100 {
        preview = preview.chars().take(99).collect::<String>() + "…";
    }
    Ok(format!(
        "Descargando {} vídeo(s) en {}.",
        candidates.len(),
        preview
    ))
}

fn download_one(url: &str, format: &str, dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let template = format!(
        "{}/%(uploader|Unknown)s/%(title).150B [%(id)s].%(ext)s",
        dir.display()
    );
    let output = Command::new("yt-dlp")
        .args([
            "--no-playlist",
            "--no-warnings",
            "--no-progress",
            "--newline",
            "--print",
            "after_move:filepath",
            "--format",
            format,
            "--output",
            &template,
        ])
        .arg(url)
        .output()
        .context("no se pudo ejecutar yt-dlp")?;
    if !output.status.success() {
        return Err(anyhow!(
            "yt-dlp falló: {}",
            clean_error(&String::from_utf8_lossy(&output.stderr))
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut found = None;
    for line in stdout.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let path = PathBuf::from(line);
        let path = if path.is_absolute() {
            path
        } else {
            dir.join(path)
        };
        if path.exists() {
            found = Some(path);
            break;
        }
    }
    found.ok_or_else(|| {
        anyhow!(
            "yt-dlp no indicó el fichero descargado: {}",
            clean_error(&String::from_utf8_lossy(&output.stderr))
        )
    })
}

fn apply_download(conn: &Connection, id: i64, dest: &Path) -> Result<()> {
    let meta = fs::metadata(dest)?;
    let size = meta.len() as i64;
    let inode = meta.ino() as i64;
    let dev = meta.dev() as i64;
    let mtime = meta.mtime();
    let fp = fingerprint::compute(dest, meta.len())?;
    let path = dest.to_string_lossy().to_string();
    db::set_video_downloaded(conn, id, &path, size, &fp, inode, dev, mtime)?;
    if let Ok(probe) = probe::probe(dest) {
        db::set_local_media_info(conn, id, &probe)?;
    }
    db::assign_default_if_none(conn, id)?;
    if let Some(video) = db::get_video(conn, id)? {
        let _ = thumbs::ensure_thumb(conn, &video);
    }
    Ok(())
}

pub fn worker(conn: &Connection, ids: &[i64], format: &str, dir: &Path) -> Result<()> {
    let dir_text = dir.to_string_lossy().to_string();
    for id in ids {
        let Some(video) = db::get_video(conn, *id)? else {
            notify(
                "Descarga cancelada",
                &format!("El vídeo {id} ya no existe en la base de datos."),
                true,
            );
            continue;
        };
        if db::path_exists(video.path.as_ref()) {
            db::set_download_state(conn, *id, "done", None, None, video.path.as_deref(), None)?;
            continue;
        }
        let Some(url) = video.url.as_deref() else {
            db::set_download_state(
                conn,
                *id,
                "failed",
                Some(format),
                Some(&dir_text),
                None,
                Some("el vídeo no tiene URL"),
            )?;
            notify(
                "Descarga fallida",
                &format!("{}: no tiene URL", video.title),
                true,
            );
            continue;
        };
        db::set_download_state(
            conn,
            *id,
            "downloading",
            Some(format),
            Some(&dir_text),
            None,
            None,
        )?;
        match download_one(url, format, dir) {
            Ok(dest) => match apply_download(conn, *id, &dest) {
                Ok(()) => {
                    let path = dest.to_string_lossy().to_string();
                    db::set_download_state(
                        conn,
                        *id,
                        "done",
                        Some(format),
                        Some(&dir_text),
                        Some(&path),
                        None,
                    )?;
                    notify(
                        "Descarga completada",
                        &format!("{}: {}", video.title, dest.display()),
                        false,
                    );
                }
                Err(err) => {
                    let message = clean_error(&format!("{err:#}"));
                    db::set_download_state(
                        conn,
                        *id,
                        "failed",
                        Some(format),
                        Some(&dir_text),
                        Some(&dest.to_string_lossy().to_string()),
                        Some(&message),
                    )?;
                    notify(
                        "Descarga fallida",
                        &format!("{}: {message}", video.title),
                        true,
                    );
                }
            },
            Err(err) => {
                let message = clean_error(&format!("{err:#}"));
                db::set_download_state(
                    conn,
                    *id,
                    "failed",
                    Some(format),
                    Some(&dir_text),
                    None,
                    Some(&message),
                )?;
                notify(
                    "Descarga fallida",
                    &format!("{}: {message}", video.title),
                    true,
                );
            }
        }
    }
    Ok(())
}
