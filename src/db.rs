use crate::paths::{self, now_secs};
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const VIDEO_COLUMNS: &str = "id, title, description, url, source, source_id, path, size, duration, width, height, has_video, has_audio, uploader, upload_date, raw_json, thumb_path, thumb_url, fingerprint, inode, dev, mtime, added_at, updated_at, last_played, progress, watched";

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Video {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub source: Option<String>,
    pub source_id: Option<String>,
    pub path: Option<String>,
    pub size: Option<i64>,
    pub duration: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub has_video: bool,
    pub has_audio: bool,
    pub uploader: Option<String>,
    pub upload_date: Option<String>,
    pub raw_json: Option<String>,
    pub thumb_path: Option<String>,
    pub thumb_url: Option<String>,
    pub fingerprint: Option<String>,
    pub inode: Option<i64>,
    pub dev: Option<i64>,
    pub mtime: Option<i64>,
    pub added_at: i64,
    pub updated_at: i64,
    pub last_played: Option<i64>,
    pub progress: f64,
    pub watched: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub system: bool,
}

#[derive(Debug, Clone)]
pub enum Filter {
    All,
    Category(String),
    Dir(PathBuf),
}

#[derive(Debug)]
pub struct LocalVideoData {
    pub title: String,
    pub path: String,
    pub size: i64,
    pub duration: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub has_video: bool,
    pub has_audio: bool,
    pub uploader: Option<String>,
    pub raw_json: Option<String>,
    pub fingerprint: String,
    pub inode: i64,
    pub dev: i64,
    pub mtime: i64,
    pub thumb_path: String,
}

#[derive(Debug)]
pub struct RemoteVideoData {
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub source: Option<String>,
    pub source_id: Option<String>,
    pub duration: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub uploader: Option<String>,
    pub upload_date: Option<String>,
    pub raw_json: Option<String>,
    pub thumb_url: Option<String>,
}

fn video_from_row(row: &Row<'_>) -> rusqlite::Result<Video> {
    Ok(Video {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        url: row.get(3)?,
        source: row.get(4)?,
        source_id: row.get(5)?,
        path: row.get(6)?,
        size: row.get(7)?,
        duration: row.get(8)?,
        width: row.get(9)?,
        height: row.get(10)?,
        has_video: row.get::<_, i64>(11)? != 0,
        has_audio: row.get::<_, i64>(12)? != 0,
        uploader: row.get(13)?,
        upload_date: row.get(14)?,
        raw_json: row.get(15)?,
        thumb_path: row.get(16)?,
        thumb_url: row.get(17)?,
        fingerprint: row.get(18)?,
        inode: row.get(19)?,
        dev: row.get(20)?,
        mtime: row.get(21)?,
        added_at: row.get(22)?,
        updated_at: row.get(23)?,
        last_played: row.get(24)?,
        progress: row.get(25)?,
        watched: row.get::<_, i64>(26)? != 0,
    })
}

fn video_query(conn: &Connection, sql: &str, params: impl rusqlite::Params) -> Result<Vec<Video>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, video_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn open() -> Result<Connection> {
    paths::ensure_dirs()?;
    let path = paths::db_path()?;
    let conn = Connection::open(&path)
        .with_context(|| format!("no se pudo abrir la base de datos {}", path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;",
    )?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER REFERENCES categories(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            system INTEGER NOT NULL DEFAULT 0
         );
         CREATE UNIQUE INDEX IF NOT EXISTS categories_parent_name
            ON categories(parent_id, name);
         CREATE UNIQUE INDEX IF NOT EXISTS categories_root_name
            ON categories(name) WHERE parent_id IS NULL;

         CREATE TABLE IF NOT EXISTS videos (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            url TEXT,
            source TEXT,
            source_id TEXT,
            path TEXT,
            size INTEGER,
            duration REAL,
            width INTEGER,
            height INTEGER,
            has_video INTEGER NOT NULL DEFAULT 1,
            has_audio INTEGER NOT NULL DEFAULT 1,
            uploader TEXT,
            upload_date TEXT,
            raw_json TEXT,
            thumb_path TEXT,
            thumb_url TEXT,
            fingerprint TEXT,
            inode INTEGER,
            dev INTEGER,
            mtime INTEGER,
            added_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            last_played INTEGER,
            progress REAL NOT NULL DEFAULT 0,
            watched INTEGER NOT NULL DEFAULT 0
         );
         CREATE UNIQUE INDEX IF NOT EXISTS videos_path ON videos(path)
            WHERE path IS NOT NULL;
         CREATE UNIQUE INDEX IF NOT EXISTS videos_source ON videos(source, source_id)
            WHERE source_id IS NOT NULL;
         CREATE INDEX IF NOT EXISTS videos_fingerprint ON videos(size, fingerprint);

         CREATE TABLE IF NOT EXISTS video_categories (
            video_id INTEGER NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
            category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
            PRIMARY KEY (video_id, category_id)
         );
         CREATE INDEX IF NOT EXISTS video_categories_category
            ON video_categories(category_id);

         CREATE TABLE IF NOT EXISTS downloads (
            video_id INTEGER PRIMARY KEY REFERENCES videos(id) ON DELETE CASCADE,
            state TEXT NOT NULL,
            format TEXT,
            dir TEXT,
            dest TEXT,
            error TEXT,
            started_at INTEGER,
            finished_at INTEGER
         );",
    )?;

    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM categories WHERE parent_id IS NULL AND name = 'Sin categorizar'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        conn.execute(
            "INSERT INTO categories(parent_id, name, system) VALUES (NULL, 'Sin categorizar', 1)",
            [],
        )?;
    }
    Ok(())
}

pub fn default_category_id(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT id FROM categories WHERE parent_id IS NULL AND name = 'Sin categorizar'",
        [],
        |row| row.get(0),
    )
    .map_err(|e| anyhow!("no existe la categoría 'Sin categorizar': {e}"))
}

pub fn all_categories(conn: &Connection) -> Result<Vec<Category>> {
    let mut stmt =
        conn.prepare("SELECT id, parent_id, name, system FROM categories ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            name: row.get(2)?,
            system: row.get::<_, i64>(3)? != 0,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn category_paths(conn: &Connection) -> Result<Vec<(i64, String, bool)>> {
    let cats = all_categories(conn)?;
    let by_id: HashMap<i64, &Category> = cats.iter().map(|c| (c.id, c)).collect();
    let mut cache: HashMap<i64, String> = HashMap::new();

    fn build(
        id: i64,
        by_id: &HashMap<i64, &Category>,
        cache: &mut HashMap<i64, String>,
        visiting: &mut HashSet<i64>,
    ) -> String {
        if let Some(value) = cache.get(&id) {
            return value.clone();
        }
        let Some(cat) = by_id.get(&id) else {
            return String::new();
        };
        if !visiting.insert(id) {
            return cat.name.clone();
        }
        let path = match cat.parent_id {
            Some(parent) if by_id.contains_key(&parent) => {
                format!("{}/{}", build(parent, by_id, cache, visiting), cat.name)
            }
            _ => cat.name.clone(),
        };
        visiting.remove(&id);
        cache.insert(id, path.clone());
        path
    }

    let mut out = Vec::new();
    for cat in &cats {
        let mut visiting = HashSet::new();
        let path = build(cat.id, &by_id, &mut cache, &mut visiting);
        out.push((cat.id, path, cat.system));
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}

pub fn resolve_category(conn: &Connection, path: &str) -> Result<Option<i64>> {
    let mut parts = path
        .trim()
        .trim_matches('/')
        .split('/')
        .filter(|p| !p.is_empty());
    let Some(first) = parts.next() else {
        return Ok(None);
    };
    let mut id: Option<i64> = conn
        .query_row(
            "SELECT id FROM categories WHERE parent_id IS NULL AND name = ?1",
            params![first],
            |row| row.get(0),
        )
        .optional()?;
    for part in parts {
        let Some(parent) = id else {
            return Ok(None);
        };
        id = conn
            .query_row(
                "SELECT id FROM categories WHERE parent_id = ?1 AND name = ?2",
                params![parent, part],
                |row| row.get(0),
            )
            .optional()?;
    }
    Ok(id)
}

pub fn category_descendants(conn: &Connection, id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "WITH RECURSIVE tree(id) AS (
            SELECT ?1
            UNION ALL
            SELECT c.id FROM categories c JOIN tree t ON c.parent_id = t.id
         )
         SELECT id FROM tree",
    )?;
    let rows = stmt.query_map(params![id], |row| row.get(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn create_category_path(conn: &Connection, path: &str) -> Result<(i64, bool)> {
    let parts: Vec<&str> = path
        .trim()
        .trim_matches('/')
        .split('/')
        .filter(|p| !p.trim().is_empty())
        .collect();
    if parts.is_empty() {
        return Err(anyhow!("ruta de categoría vacía"));
    }
    if parts.len() == 1 && parts[0] == "Sin categorizar" {
        return Ok((default_category_id(conn)?, false));
    }
    if parts.iter().any(|p| *p == "Sin categorizar") {
        return Err(anyhow!("'Sin categorizar' es una categoría reservada"));
    }

    let mut parent: Option<i64> = None;
    let mut created = false;
    for (index, part) in parts.iter().enumerate() {
        let name = part.trim();
        if name.is_empty() {
            return Err(anyhow!("nombre de categoría vacío"));
        }
        let existing: Option<i64> = match parent {
            Some(parent_id) => conn
                .query_row(
                    "SELECT id FROM categories WHERE parent_id = ?1 AND name = ?2",
                    params![parent_id, name],
                    |row| row.get(0),
                )
                .optional()?,
            None => conn
                .query_row(
                    "SELECT id FROM categories WHERE parent_id IS NULL AND name = ?1",
                    params![name],
                    |row| row.get(0),
                )
                .optional()?,
        };
        let id = match existing {
            Some(id) => id,
            None => {
                conn.execute(
                    "INSERT INTO categories(parent_id, name, system) VALUES (?1, ?2, 0)",
                    params![parent, name],
                )?;
                created = true;
                conn.last_insert_rowid()
            }
        };
        parent = Some(id);
        if index + 1 == parts.len() {
            return Ok((id, created));
        }
    }
    Err(anyhow!("ruta de categoría inválida"))
}

pub fn rename_category(conn: &Connection, id: i64, new_name: &str) -> Result<()> {
    let new_name = new_name.trim();
    if new_name.is_empty() || new_name.contains('/') {
        return Err(anyhow!("nombre de categoría inválido"));
    }
    let system: i64 = conn.query_row(
        "SELECT system FROM categories WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    if system != 0 {
        return Err(anyhow!("la categoría reservada no se puede renombrar"));
    }
    conn.execute(
        "UPDATE categories SET name = ?1 WHERE id = ?2",
        params![new_name, id],
    )?;
    Ok(())
}

pub fn move_category(conn: &Connection, id: i64, new_parent: Option<i64>) -> Result<()> {
    let system: i64 = conn.query_row(
        "SELECT system FROM categories WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    if system != 0 {
        return Err(anyhow!("la categoría reservada no se puede mover"));
    }
    if Some(id) == new_parent {
        return Err(anyhow!("una categoría no puede ser su propio padre"));
    }
    if let Some(parent) = new_parent {
        let descendants = category_descendants(conn, id)?;
        if descendants.contains(&parent) {
            return Err(anyhow!(
                "no se puede mover una categoría dentro de sí misma"
            ));
        }
    }
    conn.execute(
        "UPDATE categories SET parent_id = ?1 WHERE id = ?2",
        params![new_parent, id],
    )?;
    Ok(())
}

pub fn delete_category(conn: &Connection, id: i64) -> Result<usize> {
    let system: i64 = conn.query_row(
        "SELECT system FROM categories WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    if system != 0 {
        return Err(anyhow!("la categoría reservada no se puede borrar"));
    }
    let ids = category_descendants(conn, id)?;
    let placeholders = std::iter::repeat("?")
        .take(ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let mut affected: Vec<i64> = Vec::new();
    {
        let sql = format!(
            "SELECT DISTINCT video_id FROM video_categories WHERE category_id IN ({placeholders})"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), |row| {
            row.get::<_, i64>(0)
        })?;
        for row in rows {
            affected.push(row?);
        }
    }
    conn.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
    let default_id = default_category_id(conn)?;
    for video_id in affected {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM video_categories WHERE video_id = ?1",
            params![video_id],
            |row| row.get(0),
        )?;
        if count == 0 {
            conn.execute(
                "INSERT OR IGNORE INTO video_categories(video_id, category_id) VALUES (?1, ?2)",
                params![video_id, default_id],
            )?;
        }
    }
    Ok(ids.len())
}

pub fn assign_category(conn: &Connection, video_id: i64, category_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO video_categories(video_id, category_id) VALUES (?1, ?2)",
        params![video_id, category_id],
    )?;
    Ok(())
}

pub fn unassign_category(conn: &Connection, video_id: i64, category_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM video_categories WHERE video_id = ?1 AND category_id = ?2",
        params![video_id, category_id],
    )?;
    Ok(())
}

pub fn assign_category_preferred(conn: &Connection, video_id: i64, category_id: i64) -> Result<()> {
    let default_id = default_category_id(conn)?;
    if category_id == default_id {
        conn.execute(
            "DELETE FROM video_categories WHERE video_id = ?1 AND category_id <> ?2",
            params![video_id, default_id],
        )?;
    } else {
        unassign_category(conn, video_id, default_id)?;
    }
    assign_category(conn, video_id, category_id)
}

pub fn assign_default_if_none(conn: &Connection, video_id: i64) -> Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM video_categories WHERE video_id = ?1",
        params![video_id],
        |row| row.get(0),
    )?;
    if count == 0 {
        assign_category(conn, video_id, default_category_id(conn)?)?;
    }
    Ok(())
}

pub fn effective_categories(conn: &Connection, video_id: i64) -> Result<Vec<String>> {
    let cats = all_categories(conn)?;
    let by_id: HashMap<i64, &Category> = cats.iter().map(|c| (c.id, c)).collect();
    let mut explicit = Vec::new();
    {
        let mut stmt =
            conn.prepare("SELECT category_id FROM video_categories WHERE video_id = ?1")?;
        let rows = stmt.query_map(params![video_id], |row| row.get(0))?;
        for row in rows {
            explicit.push(row?);
        }
    }
    let mut names: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for id in explicit {
        let mut current = Some(id);
        let mut chain = Vec::new();
        while let Some(cat_id) = current {
            if !seen.insert(cat_id) {
                break;
            }
            let Some(cat) = by_id.get(&cat_id) else {
                break;
            };
            chain.push(cat.name.clone());
            current = cat.parent_id;
        }
        chain.reverse();
        if !chain.is_empty() {
            names.push(chain.join("/"));
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}

pub fn list_videos(conn: &Connection, filter: &Filter) -> Result<Vec<Video>> {
    match filter {
        Filter::All => video_query(
            conn,
            &format!("SELECT {VIDEO_COLUMNS} FROM videos ORDER BY added_at DESC, id DESC"),
            [],
        ),
        Filter::Category(path) => {
            let Some(id) = resolve_category(conn, path)? else {
                return Ok(Vec::new());
            };
            let ids = category_descendants(conn, id)?;
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            let placeholders = std::iter::repeat("?")
                .take(ids.len())
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "SELECT {VIDEO_COLUMNS} FROM videos
                 WHERE id IN (SELECT video_id FROM video_categories WHERE category_id IN ({placeholders}))
                 ORDER BY added_at DESC, id DESC"
            );
            video_query(conn, &sql, rusqlite::params_from_iter(ids.iter()))
        }
        Filter::Dir(dir) => {
            let dir = dir.to_string_lossy();
            let escaped = dir
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            let pattern = format!("{}/%", escaped.trim_end_matches('/'));
            let sql = format!(
                "SELECT {VIDEO_COLUMNS} FROM videos
                 WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\'
                 ORDER BY added_at DESC, id DESC"
            );
            video_query(conn, &sql, params![dir.trim_end_matches('/'), pattern])
        }
    }
}

pub fn all_local_dirs(conn: &Connection) -> Result<Vec<PathBuf>> {
    let mut stmt = conn.prepare("SELECT path FROM videos WHERE path IS NOT NULL")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut dirs = HashSet::new();
    for row in rows {
        let path = PathBuf::from(row?);
        if let Some(parent) = path.parent() {
            dirs.insert(parent.to_path_buf());
        }
    }
    let mut out: Vec<PathBuf> = dirs.into_iter().collect();
    out.sort();
    Ok(out)
}

pub fn get_video(conn: &Connection, id: i64) -> Result<Option<Video>> {
    let sql = format!("SELECT {VIDEO_COLUMNS} FROM videos WHERE id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![id], video_from_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn find_by_path(conn: &Connection, path: &str) -> Result<Option<Video>> {
    let sql = format!("SELECT {VIDEO_COLUMNS} FROM videos WHERE path = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![path], video_from_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn find_by_thumb(conn: &Connection, thumb: &str) -> Result<Option<Video>> {
    let sql = format!("SELECT {VIDEO_COLUMNS} FROM videos WHERE thumb_path = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![thumb], video_from_row)?;
    if let Some(row) = rows.next() {
        return Ok(Some(row?));
    }
    let stem = Path::new(thumb)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if let Ok(id) = stem.parse::<i64>() {
        return get_video(conn, id);
    }
    Ok(None)
}

pub fn find_by_source(conn: &Connection, source: &str, source_id: &str) -> Result<Option<Video>> {
    let sql = format!("SELECT {VIDEO_COLUMNS} FROM videos WHERE source = ?1 AND source_id = ?2");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![source, source_id], video_from_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn find_by_url(conn: &Connection, url: &str) -> Result<Option<Video>> {
    let sql = format!("SELECT {VIDEO_COLUMNS} FROM videos WHERE url = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![url], video_from_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn move_candidates(conn: &Connection, size: i64, fingerprint: &str) -> Result<Vec<Video>> {
    let sql = format!("SELECT {VIDEO_COLUMNS} FROM videos WHERE size = ?1 AND fingerprint = ?2");
    video_query(conn, &sql, params![size, fingerprint])
}

pub fn inode_candidates(conn: &Connection, dev: i64, inode: i64) -> Result<Vec<Video>> {
    let sql = format!("SELECT {VIDEO_COLUMNS} FROM videos WHERE dev = ?1 AND inode = ?2");
    video_query(conn, &sql, params![dev, inode])
}

pub fn insert_local(conn: &Connection, data: &LocalVideoData) -> Result<i64> {
    let now = now_secs();
    conn.execute(
        "INSERT INTO videos(
            title, path, size, duration, width, height, has_video, has_audio,
            uploader, raw_json, thumb_path, fingerprint, inode, dev, mtime,
            added_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            data.title,
            data.path,
            data.size,
            data.duration,
            data.width,
            data.height,
            data.has_video as i64,
            data.has_audio as i64,
            data.uploader,
            data.raw_json,
            data.thumb_path,
            data.fingerprint,
            data.inode,
            data.dev,
            data.mtime,
            now,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_local_metadata(
    conn: &Connection,
    id: i64,
    probe: &crate::probe::Probe,
    path: &str,
    fingerprint: &str,
    inode: i64,
    dev: i64,
    mtime: i64,
    size: i64,
) -> Result<()> {
    let now = now_secs();
    conn.execute(
        "UPDATE videos SET
            title = CASE WHEN title = '' THEN ?2 ELSE title END,
            path = ?3,
            size = ?4,
            duration = ?5,
            width = ?6,
            height = ?7,
            has_video = ?8,
            has_audio = ?9,
            uploader = COALESCE(uploader, ?10),
            raw_json = ?11,
            fingerprint = ?12,
            inode = ?13,
            dev = ?14,
            mtime = ?15,
            updated_at = ?16
         WHERE id = ?1",
        params![
            id,
            probe.title,
            path,
            size,
            probe.duration,
            probe.width,
            probe.height,
            probe.has_video as i64,
            probe.has_audio as i64,
            probe.uploader,
            probe.raw_json,
            fingerprint,
            inode,
            dev,
            mtime,
            now
        ],
    )?;
    Ok(())
}

pub fn update_video_path(
    conn: &Connection,
    id: i64,
    path: &str,
    fingerprint: &str,
    inode: i64,
    dev: i64,
    mtime: i64,
    size: i64,
) -> Result<()> {
    let now = now_secs();
    conn.execute(
        "UPDATE videos SET path = ?2, fingerprint = ?3, inode = ?4, dev = ?5,
            mtime = ?6, size = ?7, updated_at = ?8 WHERE id = ?1",
        params![id, path, fingerprint, inode, dev, mtime, size, now],
    )?;
    Ok(())
}

pub fn set_thumb(conn: &Connection, id: i64, thumb_path: &str) -> Result<()> {
    conn.execute(
        "UPDATE videos SET thumb_path = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, thumb_path, now_secs()],
    )?;
    Ok(())
}

pub fn upsert_remote(conn: &Connection, data: &RemoteVideoData) -> Result<(i64, bool)> {
    let existing = if let (Some(source), Some(source_id)) = (&data.source, &data.source_id) {
        find_by_source(conn, source, source_id)?
    } else {
        None
    };
    let existing = match existing {
        Some(video) => Some(video),
        None => find_by_url(conn, &data.url)?,
    };
    let now = now_secs();
    if let Some(video) = existing {
        conn.execute(
            "UPDATE videos SET
                title = ?2,
                description = ?3,
                url = ?4,
                source = COALESCE(?5, source),
                source_id = COALESCE(?6, source_id),
                duration = ?7,
                width = ?8,
                height = ?9,
                uploader = ?10,
                upload_date = ?11,
                raw_json = ?12,
                thumb_url = COALESCE(?13, thumb_url),
                updated_at = ?14
             WHERE id = ?1",
            params![
                video.id,
                data.title,
                data.description,
                data.url,
                data.source,
                data.source_id,
                data.duration,
                data.width,
                data.height,
                data.uploader,
                data.upload_date,
                data.raw_json,
                data.thumb_url,
                now
            ],
        )?;
        Ok((video.id, false))
    } else {
        conn.execute(
            "INSERT INTO videos(
                title, description, url, source, source_id, duration, width, height,
                has_video, has_audio, uploader, upload_date, raw_json, thumb_url,
                added_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, 1, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                data.title,
                data.description,
                data.url,
                data.source,
                data.source_id,
                data.duration,
                data.width,
                data.height,
                data.uploader,
                data.upload_date,
                data.raw_json,
                data.thumb_url,
                now,
                now
            ],
        )?;
        Ok((conn.last_insert_rowid(), true))
    }
}

pub fn delete_video(conn: &Connection, id: i64) -> Result<Option<String>> {
    let thumb: Option<String> = conn
        .query_row(
            "SELECT thumb_path FROM videos WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    conn.execute("DELETE FROM videos WHERE id = ?1", params![id])?;
    Ok(thumb)
}

pub fn set_progress(
    conn: &Connection,
    id: i64,
    pos: f64,
    duration: Option<f64>,
    force_watched: bool,
) -> Result<()> {
    let progress = pos.max(0.0);
    let mut watched: i64 = conn.query_row(
        "SELECT watched FROM videos WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    if force_watched {
        watched = 1;
    } else if let Some(duration) = duration {
        if duration > 0.0 && progress / duration >= 0.9 {
            watched = 1;
        }
    }
    conn.execute(
        "UPDATE videos SET progress = ?2, watched = ?3, last_played = ?4 WHERE id = ?1",
        params![id, progress, watched, now_secs()],
    )?;
    Ok(())
}

pub fn set_download_state(
    conn: &Connection,
    video_id: i64,
    state: &str,
    format: Option<&str>,
    dir: Option<&str>,
    dest: Option<&str>,
    error: Option<&str>,
) -> Result<()> {
    let now = now_secs();
    conn.execute(
        "INSERT INTO downloads(video_id, state, format, dir, dest, error, started_at, finished_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(video_id) DO UPDATE SET
            state = excluded.state,
            format = COALESCE(excluded.format, downloads.format),
            dir = COALESCE(excluded.dir, downloads.dir),
            dest = COALESCE(excluded.dest, downloads.dest),
            error = excluded.error,
            started_at = COALESCE(downloads.started_at, excluded.started_at),
            finished_at = excluded.finished_at",
        params![
            video_id,
            state,
            format,
            dir,
            dest,
            error,
            now,
            if state == "done" || state == "failed" {
                Some(now)
            } else {
                None
            }
        ],
    )?;
    Ok(())
}

pub fn download_state(
    conn: &Connection,
    video_id: i64,
) -> Result<Option<(String, Option<String>)>> {
    conn.query_row(
        "SELECT state, error FROM downloads WHERE video_id = ?1",
        params![video_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(Into::into)
}

pub fn set_video_downloaded(
    conn: &Connection,
    id: i64,
    path: &str,
    size: i64,
    fingerprint: &str,
    inode: i64,
    dev: i64,
    mtime: i64,
) -> Result<()> {
    let now = now_secs();
    conn.execute(
        "UPDATE videos SET path = ?2, size = ?3, fingerprint = ?4, inode = ?5,
            dev = ?6, mtime = ?7, has_video = 1, updated_at = ?8 WHERE id = ?1",
        params![id, path, size, fingerprint, inode, dev, mtime, now],
    )?;
    Ok(())
}

pub fn set_local_media_info(conn: &Connection, id: i64, probe: &crate::probe::Probe) -> Result<()> {
    conn.execute(
        "UPDATE videos SET duration = ?2, width = ?3, height = ?4,
            has_video = ?5, has_audio = ?6, updated_at = ?7 WHERE id = ?1",
        params![
            id,
            probe.duration,
            probe.width,
            probe.height,
            probe.has_video as i64,
            probe.has_audio as i64,
            now_secs()
        ],
    )?;
    Ok(())
}

pub fn path_exists(path: Option<&String>) -> bool {
    path.map(|p| Path::new(p).exists()).unwrap_or(false)
}
