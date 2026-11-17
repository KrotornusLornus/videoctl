use crate::db::{self, RemoteVideoData};
use crate::thumbs;
use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

fn value_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn value_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|v| v.as_i64()).or_else(|| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    })
}

fn value_f64(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|v| v.as_f64()).or_else(|| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    })
}

fn best_thumbnail(value: &Value) -> Option<String> {
    if let Some(url) = value_string(value, "thumbnail") {
        return Some(url);
    }
    let mut best: Option<(i64, String)> = None;
    if let Some(items) = value.get("thumbnails").and_then(|v| v.as_array()) {
        for item in items {
            let Some(url) = item.get("url").and_then(|v| v.as_str()) else {
                continue;
            };
            let width = item.get("width").and_then(|v| v.as_i64()).unwrap_or(0);
            if best.as_ref().map(|(w, _)| width >= *w).unwrap_or(true) {
                best = Some((width, url.to_string()));
            }
        }
    }
    best.map(|(_, url)| url)
}

pub fn fetch_json(url: &str) -> Result<Value> {
    let output = Command::new("yt-dlp")
        .args([
            "-J",
            "--no-playlist",
            "--no-warnings",
            "--no-progress",
            "--skip-download",
        ])
        .arg(url)
        .output()
        .context("no se pudo ejecutar yt-dlp")?;
    if !output.status.success() {
        return Err(anyhow!(
            "yt-dlp falló: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    serde_json::from_str(&text).context("yt-dlp devolvió JSON inválido")
}

pub fn import_url(conn: &Connection, url: &str) -> Result<String> {
    let json = fetch_json(url)?;
    if json.get("_type").and_then(|v| v.as_str()) == Some("playlist") {
        return Err(anyhow!("las playlists no están soportadas por ahora"));
    }
    let title = value_string(&json, "title")
        .or_else(|| value_string(&json, "fulltitle"))
        .unwrap_or_else(|| "Sin título".to_string());
    let page_url = value_string(&json, "webpage_url")
        .or_else(|| value_string(&json, "original_url"))
        .unwrap_or_else(|| url.to_string());
    let source = value_string(&json, "extractor_key").or_else(|| value_string(&json, "extractor"));
    let source_id = value_string(&json, "id");
    let uploader = value_string(&json, "uploader")
        .or_else(|| value_string(&json, "channel"))
        .or_else(|| value_string(&json, "creator"));
    let description = value_string(&json, "description");
    let upload_date = value_string(&json, "upload_date");
    let thumb_url = best_thumbnail(&json);
    let raw_json = serde_json::to_string(&json).ok();

    let data = RemoteVideoData {
        title,
        description,
        url: page_url,
        source,
        source_id,
        duration: value_f64(&json, "duration"),
        width: value_i64(&json, "width"),
        height: value_i64(&json, "height"),
        uploader,
        upload_date,
        raw_json,
        thumb_url,
    };
    let (id, created) = db::upsert_remote(conn, &data)?;
    db::assign_default_if_none(conn, id)?;
    let video = db::get_video(conn, id)?.expect("video importado");
    let _ = thumbs::ensure_thumb(conn, &video);
    Ok(format!(
        "{}: {} (id {})",
        if created { "Importado" } else { "Actualizado" },
        data.title,
        id
    ))
}

pub fn import_urls(conn: &Connection, urls: &[String]) -> Result<()> {
    for url in urls {
        match import_url(conn, url) {
            Ok(status) => println!("{status}"),
            Err(err) => eprintln!("error: {url}: {err:#}"),
        }
    }
    Ok(())
}

pub fn import_file(conn: &Connection, file: &Path) -> Result<()> {
    let content =
        fs::read_to_string(file).with_context(|| format!("no se pudo leer {}", file.display()))?;
    let urls: Vec<String> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    if urls.is_empty() {
        println!("No hay URLs para importar.");
    } else {
        println!("Importando {} URL(s)...", urls.len());
        import_urls(conn, &urls)?;
    }
    Ok(())
}
