use crate::db::{self, LocalVideoData};
use crate::fingerprint;
use crate::paths;
use crate::probe;
use crate::thumbs;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct ScanStats {
    pub new: usize,
    pub moved: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: usize,
}

fn is_missing(path: Option<&String>) -> bool {
    path.map(|p| !Path::new(p).exists()).unwrap_or(true)
}

fn choose_moved(candidates: Vec<db::Video>, current: &str) -> Option<db::Video> {
    candidates
        .into_iter()
        .find(|v| v.path.as_deref() != Some(current) && is_missing(v.path.as_ref()))
}

pub fn scan_dir(conn: &Connection, dir: &Path) -> Result<ScanStats> {
    let dir = paths::canonical_dir(dir)?;
    let mut stats = ScanStats::default();

    for entry in WalkDir::new(&dir)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() || !paths::is_video_extension(entry.path()) {
            continue;
        }
        match scan_file(conn, entry.path()) {
            Ok(Some(kind)) => match kind {
                ScanKind::New => stats.new += 1,
                ScanKind::Moved => stats.moved += 1,
                ScanKind::Updated => stats.updated += 1,
            },
            Ok(None) => stats.skipped += 1,
            Err(err) => {
                stats.errors += 1;
                eprintln!("aviso: {}: {err:#}", entry.path().display());
            }
        }
    }
    Ok(stats)
}

#[derive(Debug, Clone, Copy)]
enum ScanKind {
    New,
    Moved,
    Updated,
}

fn scan_file(conn: &Connection, file: &Path) -> Result<Option<ScanKind>> {
    let canonical = fs::canonicalize(file)
        .with_context(|| format!("no se pudo resolver {}", file.display()))?;
    let path = canonical.to_string_lossy().to_string();
    let meta = fs::metadata(&canonical)?;
    let size = meta.len() as i64;
    let inode = meta.ino() as i64;
    let dev = meta.dev() as i64;
    let mtime = meta.mtime();

    if let Some(existing) = db::find_by_path(conn, &path)? {
        db::update_video_path(
            conn,
            existing.id,
            &path,
            existing.fingerprint.as_deref().unwrap_or(""),
            inode,
            dev,
            mtime,
            size,
        )?;
        if existing.mtime.map(|m| m != mtime).unwrap_or(true) {
            if let Ok(probe) = probe::probe(&canonical) {
                let fp = fingerprint::compute(&canonical, meta.len())?;
                db::update_local_metadata(
                    conn,
                    existing.id,
                    &probe,
                    &path,
                    &fp,
                    inode,
                    dev,
                    mtime,
                    size,
                )?;
                if existing.has_video {
                    let video =
                        db::get_video(conn, existing.id)?.expect("video recién actualizado");
                    let _ = thumbs::regenerate_local(conn, &video);
                }
                return Ok(Some(ScanKind::Updated));
            }
        }
        if existing
            .thumb_path
            .as_deref()
            .map(|t| !Path::new(t).exists())
            .unwrap_or(true)
        {
            let video = db::get_video(conn, existing.id)?.expect("video existente");
            let _ = thumbs::ensure_thumb(conn, &video);
        }
        return Ok(None);
    }

    let fp = fingerprint::compute(&canonical, meta.len())?;

    let inode_moved = choose_moved(db::inode_candidates(conn, dev, inode)?, &path);
    if let Some(video) = inode_moved {
        db::update_video_path(conn, video.id, &path, &fp, inode, dev, mtime, size)?;
        if video
            .thumb_path
            .as_deref()
            .map(|t| !Path::new(t).exists())
            .unwrap_or(true)
        {
            let video = db::get_video(conn, video.id)?.expect("video movido");
            let _ = thumbs::ensure_thumb(conn, &video);
        }
        return Ok(Some(ScanKind::Moved));
    }

    let fingerprint_moved = choose_moved(db::move_candidates(conn, size, &fp)?, &path);
    if let Some(video) = fingerprint_moved {
        db::update_video_path(conn, video.id, &path, &fp, inode, dev, mtime, size)?;
        if video
            .thumb_path
            .as_deref()
            .map(|t| !Path::new(t).exists())
            .unwrap_or(true)
        {
            let video = db::get_video(conn, video.id)?.expect("video movido");
            let _ = thumbs::ensure_thumb(conn, &video);
        }
        return Ok(Some(ScanKind::Moved));
    }

    let probe = probe::probe(&canonical)?;
    let target = thumbs::thumb_path_for(0)?;
    let target = target.with_file_name(format!("pending-{}.jpg", std::process::id()));
    let data = LocalVideoData {
        title: probe.title.clone(),
        path: path.clone(),
        size,
        duration: probe.duration,
        width: probe.width,
        height: probe.height,
        has_video: probe.has_video,
        has_audio: probe.has_audio,
        uploader: probe.uploader.clone(),
        raw_json: Some(probe.raw_json.clone()),
        fingerprint: fp,
        inode,
        dev,
        mtime,
        thumb_path: target.to_string_lossy().to_string(),
    };
    let id = db::insert_local(conn, &data)?;
    db::assign_default_if_none(conn, id)?;
    let video = db::get_video(conn, id)?.expect("video recién insertado");
    let _ = thumbs::regenerate_local(conn, &video);
    Ok(Some(ScanKind::New))
}
