use crate::db::{self, Video};
use anyhow::Result;
use rusqlite::Connection;
use serde_json::json;
use std::path::Path;

fn truncate(value: &str, max: usize) -> String {
    let mut out = String::new();
    for (index, character) in value.chars().enumerate() {
        if index >= max {
            out.push('…');
            break;
        }
        out.push(character);
    }
    out
}

fn format_duration(seconds: Option<f64>) -> String {
    let Some(seconds) = seconds.filter(|s| *s > 0.0) else {
        return "desconocida".to_string();
    };
    let total = seconds.round() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

fn format_size(bytes: Option<i64>) -> String {
    let Some(bytes) = bytes.filter(|s| *s > 0) else {
        return "desconocido".to_string();
    };
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < units.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", units[unit])
    }
}

fn format_date(value: &str) -> String {
    if value.len() == 8 && value.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &value[0..4], &value[4..6], &value[6..8])
    } else {
        value.to_string()
    }
}

fn resolution(video: &Video) -> String {
    match (video.width, video.height) {
        (Some(width), Some(height)) => format!("{width}x{height}"),
        (Some(width), None) => format!("{width}px"),
        (None, Some(height)) => format!("altura {height}"),
        _ => "desconocida".to_string(),
    }
}

pub fn text(conn: &Connection, video: &Video) -> Result<String> {
    let categories = db::effective_categories(conn, video.id)?;
    let local_exists = video
        .path
        .as_deref()
        .map(|path| Path::new(path).exists())
        .unwrap_or(false);
    let kind = match (&video.url, local_exists) {
        (Some(_), true) => "online + local",
        (Some(_), false) => "online",
        (None, true) => "local",
        (None, false) => "local (no encontrado)",
    };
    let progress = if video.duration.unwrap_or(0.0) > 0.0 {
        format!(
            "{:.0}%",
            (video.progress / video.duration.unwrap_or(1.0) * 100.0).clamp(0.0, 100.0)
        )
    } else if video.progress > 0.0 {
        format_duration(Some(video.progress))
    } else {
        "0%".to_string()
    };
    let download = db::download_state(conn, video.id)?;

    let mut lines = vec![
        format!("Título: {}", truncate(&video.title, 160)),
        format!(
            "Duración: {}   Resolución: {}   Tamaño: {}",
            format_duration(video.duration),
            resolution(video),
            format_size(video.size)
        ),
        format!(
            "Categorías: {}",
            if categories.is_empty() {
                "ninguna".to_string()
            } else {
                truncate(&categories.join(", "), 160)
            }
        ),
        format!("Tipo: {kind}"),
    ];
    if let Some(path) = &video.path {
        lines.push(format!(
            "Archivo: {}{}",
            truncate(path, 160),
            if local_exists {
                ""
            } else {
                "  [no encontrado]"
            }
        ));
    }
    if let Some(url) = &video.url {
        lines.push(format!("URL: {}", truncate(url, 160)));
    }
    if video.uploader.is_some() || video.upload_date.is_some() {
        lines.push(format!(
            "Subido por: {}   Fecha: {}",
            video.uploader.as_deref().unwrap_or("desconocido"),
            video
                .upload_date
                .as_deref()
                .map(format_date)
                .unwrap_or_else(|| "desconocida".to_string())
        ));
    }
    lines.push(format!(
        "Progreso: {}   Visto: {}",
        progress,
        if video.watched { "sí" } else { "no" }
    ));
    if let Some((state, error)) = download {
        let detail = error
            .filter(|e| !e.trim().is_empty())
            .map(|e| format!(": {}", truncate(&e, 120)))
            .unwrap_or_default();
        lines.push(format!("Descarga: {state}{detail}"));
    }
    Ok(lines.join("\n"))
}

pub fn json(conn: &Connection, video: &Video) -> Result<String> {
    let categories = db::effective_categories(conn, video.id)?;
    let local_exists = video
        .path
        .as_deref()
        .map(|path| Path::new(path).exists())
        .unwrap_or(false);
    let download = db::download_state(conn, video.id)?;
    let value = json!({
        "id": video.id,
        "title": video.title,
        "url": video.url,
        "source": video.source,
        "source_id": video.source_id,
        "path": video.path,
        "local_exists": local_exists,
        "size": video.size,
        "duration": video.duration,
        "width": video.width,
        "height": video.height,
        "has_video": video.has_video,
        "has_audio": video.has_audio,
        "uploader": video.uploader,
        "upload_date": video.upload_date,
        "categories": categories,
        "progress": video.progress,
        "watched": video.watched,
        "last_played": video.last_played,
        "download": download.map(|(state, error)| json!({"state": state, "error": error})),
    });
    Ok(serde_json::to_string_pretty(&value)?)
}
