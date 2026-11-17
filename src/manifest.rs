use crate::db::Video;
use anyhow::{bail, Result};
use serde_json::json;
use std::path::Path;

fn source_for(video: &Video) -> Option<String> {
    if let Some(path) = &video.path {
        if Path::new(path).exists() {
            return Some(path.clone());
        }
    }
    video.url.clone()
}

fn resume_position(video: &Video) -> f64 {
    if video.watched || video.progress <= 10.0 {
        0.0
    } else {
        video.progress
    }
}

pub fn sources(videos: &[Video]) -> Result<Vec<String>> {
    let sources: Vec<String> = videos.iter().filter_map(source_for).collect();
    if sources.is_empty() {
        bail!("no hay nada reproducible");
    }
    Ok(sources)
}

pub fn render(videos: &[Video]) -> Result<String> {
    let mut items = Vec::new();
    for video in videos {
        let Some(source) = source_for(video) else {
            eprintln!(
                "aviso: {} no tiene fichero local ni URL reproducible",
                video.title
            );
            continue;
        };
        items.push(json!({
            "id": video.id,
            "title": &video.title,
            "source": source,
            "resume": resume_position(video),
            "duration": video.duration,
        }));
    }
    if items.is_empty() {
        bail!("no hay nada reproducible");
    }
    Ok(serde_json::to_string(&json!({ "items": items }))?)
}
