use crate::db::{self, Video};
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::collections::HashSet;

pub fn one(conn: &Connection, id: Option<i64>, thumb: Option<&str>) -> Result<Video> {
    if let Some(id) = id {
        return db::get_video(conn, id)?.ok_or_else(|| anyhow!("no existe el vídeo {id}"));
    }
    if let Some(thumb) = thumb {
        return db::find_by_thumb(conn, thumb)?
            .ok_or_else(|| anyhow!("no se encontró el vídeo para {thumb}"));
    }
    Err(anyhow!("indica --id o --thumb"))
}

pub fn by_thumbs(conn: &Connection, thumbs: &[String]) -> Result<Vec<Video>> {
    let mut seen = HashSet::new();
    let mut videos = Vec::new();
    for thumb in thumbs {
        if let Some(video) = db::find_by_thumb(conn, thumb)? {
            if seen.insert(video.id) {
                videos.push(video);
            }
        }
    }
    if videos.is_empty() {
        return Err(anyhow!("no se encontró ningún vídeo"));
    }
    Ok(videos)
}
