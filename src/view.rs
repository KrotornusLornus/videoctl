use crate::db::{self, Filter};
use crate::thumbs;
use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn list_thumbs(conn: &Connection, filter: &Filter) -> Result<Vec<String>> {
    let videos = db::list_videos(conn, filter)?;
    let mut result = Vec::new();
    for video in videos {
        if let Some(thumb) = video.thumb_path.as_deref() {
            if Path::new(thumb).is_file() {
                result.push(thumb.to_string());
                continue;
            }
        }
        match thumbs::ensure_fallback(video.id) {
            Ok(fallback) => result.push(fallback.to_string_lossy().to_string()),
            Err(err) => eprintln!("error: fallback de {}: {err:#}", video.title),
        }
    }
    Ok(result)
}
