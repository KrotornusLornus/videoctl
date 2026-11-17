use crate::db;
use crate::lookup;
use crate::paths;
use crate::thumbs;
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn trim_category(value: &str) -> String {
    value.trim().trim_matches('/').to_string()
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

pub fn categories(conn: &Connection, include_system: bool) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for (_, path, system) in db::category_paths(conn)? {
        if include_system || !system {
            result.push(path);
        }
    }
    Ok(result)
}

pub fn dirs(conn: &Connection) -> Result<Vec<PathBuf>> {
    let default = paths::default_videos_dir()?;
    let mut dirs = vec![default];
    for dir in db::all_local_dirs(conn)? {
        dirs.push(dir);
    }
    dirs.sort();
    dirs.dedup();
    Ok(dirs)
}

pub fn scan_dirs() -> Result<Vec<PathBuf>> {
    let default = paths::default_videos_dir()?;
    let mut dirs = Vec::new();
    if default.exists() {
        for entry in WalkDir::new(&default)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !is_hidden(entry))
            .filter_map(Result::ok)
        {
            if entry.file_type().is_dir() {
                dirs.push(entry.path().to_path_buf());
            }
        }
    }
    dirs.sort();
    let mut entries = vec![default];
    for dir in dirs {
        if !entries.contains(&dir) {
            entries.push(dir);
        }
    }
    Ok(entries)
}

pub fn assign(
    conn: &Connection,
    thumbs: &[String],
    category: &str,
    create: bool,
) -> Result<String> {
    let category = trim_category(category);
    if category.is_empty() {
        return Err(anyhow!("categoría vacía"));
    }
    let videos = lookup::by_thumbs(conn, thumbs)?;
    let category_id = if create {
        db::create_category_path(conn, &category)?.0
    } else {
        db::resolve_category(conn, &category)?
            .ok_or_else(|| anyhow!("la categoría {category} no existe"))?
    };
    for video in &videos {
        db::assign_category_preferred(conn, video.id, category_id)?;
    }
    Ok(format!(
        "Asignados {} vídeo(s) a {}.",
        videos.len(),
        category
    ))
}

pub fn delete(conn: &Connection, thumbs: &[String], remove_files: bool) -> Result<String> {
    let videos = lookup::by_thumbs(conn, thumbs)?;
    let mut deleted = 0usize;
    let mut files = 0usize;
    for video in &videos {
        if remove_files {
            if let Some(path) = &video.path {
                if Path::new(path).exists() && fs::remove_file(path).is_ok() {
                    files += 1;
                }
            }
        }
        if let Some(thumb) = db::delete_video(conn, video.id)? {
            thumbs::remove_thumb(Some(&thumb));
        }
        thumbs::remove_fallback(video.id);
        deleted += 1;
    }
    Ok(format!(
        "Borrados {deleted} vídeo(s) de la base de datos y {files} fichero(s)."
    ))
}

pub fn create_category(conn: &Connection, path: &str) -> Result<String> {
    let path = trim_category(path);
    if path.is_empty() {
        return Err(anyhow!("categoría vacía"));
    }
    let (_, created) = db::create_category_path(conn, &path)?;
    Ok(if created {
        format!("Categoría creada: {path}.")
    } else {
        format!("La categoría ya existía: {path}.")
    })
}

fn category_id(conn: &Connection, path: &str) -> Result<i64> {
    let path = trim_category(path);
    db::resolve_category(conn, &path)?.ok_or_else(|| anyhow!("la categoría {path} no existe"))
}

pub fn rename_category(conn: &Connection, path: &str, name: &str) -> Result<String> {
    let path = trim_category(path);
    let id = category_id(conn, &path)?;
    db::rename_category(conn, id, name)?;
    Ok(format!("Categoría renombrada: {path} → {name}."))
}

pub fn move_category(conn: &Connection, path: &str, parent: Option<&str>) -> Result<String> {
    let path = trim_category(path);
    let id = category_id(conn, &path)?;
    let parent = parent.map(trim_category).filter(|value| !value.is_empty());
    let parent_id = match parent.as_deref() {
        Some(parent) => Some(category_id(conn, parent)?),
        None => None,
    };
    db::move_category(conn, id, parent_id)?;
    Ok(match parent {
        Some(parent) => format!("Categoría movida: {path} → {parent}."),
        None => format!("Categoría movida: {path} → Raíz."),
    })
}

pub fn delete_category(conn: &Connection, path: &str) -> Result<String> {
    let path = trim_category(path);
    let id = category_id(conn, &path)?;
    let count = db::delete_category(conn, id)?;
    Ok(format!(
        "Categoría borrada: {path} (y {} subcategoría(s)). Los vídeos no se han borrado.",
        count.saturating_sub(1)
    ))
}
