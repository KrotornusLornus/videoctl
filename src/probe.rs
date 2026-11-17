use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Probe {
    pub title: String,
    pub duration: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub has_video: bool,
    pub has_audio: bool,
    pub uploader: Option<String>,
    pub raw_json: String,
}

fn as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

fn as_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

fn title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Sin título")
        .replace('_', " ")
        .trim()
        .to_string()
}

pub fn probe(path: &Path) -> Result<Probe> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .context("no se pudo ejecutar ffprobe")?;
    if !output.status.success() {
        return Err(anyhow!(
            "ffprobe falló para {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let json: Value = serde_json::from_str(&text).context("ffprobe devolvió JSON inválido")?;

    let mut duration = json
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(as_f64);
    let mut width = None;
    let mut height = None;
    let mut has_video = false;
    let mut has_audio = false;
    let mut stream_title = None;
    let mut stream_artist = None;

    if let Some(streams) = json.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            let codec_type = stream
                .get("codec_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let attached = stream
                .get("disposition")
                .and_then(|d| d.get("attached_pic"))
                .and_then(as_i64)
                .unwrap_or(0)
                != 0;
            match codec_type {
                "video" if !attached => {
                    has_video = true;
                    width = width.or_else(|| stream.get("width").and_then(as_i64));
                    height = height.or_else(|| stream.get("height").and_then(as_i64));
                    if stream_title.is_none() {
                        stream_title = stream
                            .get("tags")
                            .and_then(|t| t.get("title"))
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                    }
                    if stream_artist.is_none() {
                        stream_artist = stream
                            .get("tags")
                            .and_then(|t| t.get("artist"))
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                    }
                }
                "audio" => {
                    has_audio = true;
                }
                _ => {}
            }
            duration = duration.or_else(|| stream.get("duration").and_then(as_f64));
        }
    }

    let format_tags = json.get("format").and_then(|f| f.get("tags"));
    let title = format_tags
        .and_then(|t| t.get("title"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or(stream_title)
        .unwrap_or_else(|| title_from_path(path));
    let uploader = stream_artist.or_else(|| {
        format_tags
            .and_then(|t| t.get("artist"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    });

    let _ = fs::metadata(path)?;

    Ok(Probe {
        title,
        duration,
        width,
        height,
        has_video,
        has_audio,
        uploader,
        raw_json: text.trim().to_string(),
    })
}
