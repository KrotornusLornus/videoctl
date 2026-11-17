use crate::db::Video;
use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn parent_dir(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn collect_values(video: &Video, both: bool) -> Vec<String> {
    let mut out = Vec::new();
    if both {
        if let Some(url) = &video.url {
            out.push(url.clone());
        }
        if let Some(path) = &video.path {
            out.push(parent_dir(path));
        }
    } else if let Some(url) = &video.url {
        out.push(url.clone());
    } else if let Some(path) = &video.path {
        out.push(parent_dir(path));
    }
    out
}

pub fn copy(videos: &[Video], both: bool) -> Result<String> {
    let mut values: Vec<String> = Vec::new();
    for video in videos {
        for value in collect_values(video, both) {
            if !values.contains(&value) {
                values.push(value);
            }
        }
    }
    if values.is_empty() {
        return Err(anyhow!("no hay URL ni carpeta que copiar"));
    }

    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("no se pudo ejecutar wl-copy")?;
    {
        let stdin = child.stdin.as_mut().context("wl-copy sin stdin")?;
        stdin.write_all(values.join("\n").as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("wl-copy terminó con error"));
    }

    let mut preview = values.join(" | ");
    if preview.chars().count() > 140 {
        preview = preview.chars().take(139).collect::<String>() + "…";
    }
    Ok(format!("Copiado {} elemento(s): {}", values.len(), preview))
}
