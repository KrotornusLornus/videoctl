mod catalog;
mod copy;
mod db;
mod download;
mod fingerprint;
mod import;
mod info;
mod lookup;
mod manifest;
mod paths;
mod probe;
mod scan;
mod thumbs;
mod view;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use db::Filter;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "videoctl",
    version,
    about = "Gestor de vídeos para swayimg + mpv"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List {
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Scan {
        dir: Option<PathBuf>,
    },
    ScanDirs,
    Import {
        urls: Vec<String>,
    },
    ImportFile {
        file: PathBuf,
    },
    Info {
        #[arg(long)]
        id: Option<i64>,
        #[arg(long)]
        thumb: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Source {
        #[arg(long = "thumb")]
        thumbs: Vec<String>,
    },
    Manifest {
        #[arg(long = "thumb")]
        thumbs: Vec<String>,
    },
    Progress {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        pos: f64,
        #[arg(long)]
        duration: Option<f64>,
        #[arg(long)]
        watched: bool,
    },
    Copy {
        #[arg(long)]
        both: bool,
        #[arg(long = "thumb")]
        thumbs: Vec<String>,
    },
    Placeholder,
    Thumbs {
        #[command(subcommand)]
        command: ThumbsCommands,
    },
    Categories {
        #[arg(long)]
        no_system: bool,
    },
    Dirs,
    Assign {
        #[arg(long)]
        category: String,
        #[arg(long)]
        create: bool,
        #[arg(long = "thumb")]
        thumbs: Vec<String>,
    },
    Delete {
        #[arg(long)]
        files: bool,
        #[arg(long = "thumb")]
        thumbs: Vec<String>,
    },
    Download {
        #[arg(long)]
        format: String,
        #[arg(long)]
        dir: PathBuf,
        #[arg(long = "thumb")]
        thumbs: Vec<String>,
    },
    DownloadCandidates {
        #[arg(long = "thumb")]
        thumbs: Vec<String>,
    },
    DownloadQualities,
    Category {
        #[command(subcommand)]
        command: CategoryCommands,
    },
    DownloadWorker {
        ids: Vec<i64>,
        #[arg(long)]
        format: String,
        #[arg(long)]
        dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum ThumbsCommands {
    Ensure {
        #[arg(long, default_value = "auto")]
        method: String,
        #[arg(long = "thumb")]
        thumbs: Vec<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    Regenerate {
        #[arg(long, default_value = "auto")]
        method: String,
        #[arg(long = "thumb")]
        thumbs: Vec<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum CategoryCommands {
    Create {
        #[arg(long)]
        path: String,
    },
    Rename {
        #[arg(long)]
        path: String,
        #[arg(long)]
        name: String,
    },
    Move {
        #[arg(long)]
        path: String,
        #[arg(long)]
        parent: Option<String>,
    },
    Delete {
        #[arg(long)]
        path: String,
    },
}

fn filter_for(category: Option<String>, dir: Option<PathBuf>) -> Result<Filter> {
    if category.is_some() && dir.is_some() {
        return Err(anyhow!("usa --category o --dir, no ambos"));
    }
    if let Some(category) = category {
        let category = category.trim().trim_matches('/').to_string();
        if category.is_empty() {
            return Err(anyhow!("categoría vacía"));
        }
        return Ok(Filter::Category(category));
    }
    if let Some(dir) = dir {
        let text = dir.to_string_lossy();
        let path = paths::expand_tilde(text.as_ref())?;
        let path = if path.exists() {
            paths::canonical_dir(&path)?
        } else {
            path
        };
        return Ok(Filter::Dir(path));
    }
    Ok(Filter::All)
}

fn videos_for_thumbs(
    conn: &rusqlite::Connection,
    thumbs: &[String],
    category: Option<String>,
    dir: Option<PathBuf>,
) -> Result<Vec<db::Video>> {
    if !thumbs.is_empty() && (category.is_some() || dir.is_some()) {
        return Err(anyhow!("usa --thumb o --category/--dir, no ambos"));
    }
    if !thumbs.is_empty() {
        return lookup::by_thumbs(conn, thumbs);
    }
    let filter = filter_for(category, dir)?;
    db::list_videos(conn, &filter)
}

fn read_urls_from_stdin() -> Result<Vec<String>> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect())
}

fn print_scan(stats: &scan::ScanStats) {
    println!(
        "Escaneados: {} nuevos, {} movidos, {} actualizados, {} omitidos, {} errores.",
        stats.new, stats.moved, stats.updated, stats.skipped, stats.errors
    );
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::List {
            category,
            dir,
            json,
        } => {
            let filter = filter_for(category, dir)?;
            let conn = db::open()?;
            let thumbs = view::list_thumbs(&conn, &filter)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&thumbs)?);
            } else {
                for thumb in thumbs {
                    println!("{thumb}");
                }
            }
            Ok(())
        }
        Commands::Scan { dir } => {
            let dir = match dir {
                Some(dir) => {
                    let text = dir.to_string_lossy();
                    paths::expand_tilde(text.as_ref())?
                }
                None => paths::default_videos_dir()?,
            };
            let conn = db::open()?;
            let stats = scan::scan_dir(&conn, &dir)?;
            print_scan(&stats);
            Ok(())
        }
        Commands::ScanDirs => {
            for dir in catalog::scan_dirs()? {
                println!("{}", dir.display());
            }
            Ok(())
        }
        Commands::Import { urls } => {
            let urls = if urls.is_empty() {
                read_urls_from_stdin()?
            } else {
                urls
            };
            if urls.is_empty() {
                println!("No hay URLs para importar.");
                return Ok(());
            }
            let conn = db::open()?;
            import::import_urls(&conn, &urls)?;
            Ok(())
        }
        Commands::ImportFile { file } => {
            let conn = db::open()?;
            import::import_file(&conn, &file)
        }
        Commands::Info { id, thumb, json } => {
            let conn = db::open()?;
            let video = lookup::one(&conn, id, thumb.as_deref())?;
            if json {
                println!("{}", info::json(&conn, &video)?);
            } else {
                println!("{}", info::text(&conn, &video)?);
            }
            Ok(())
        }
        Commands::Source { thumbs } => {
            let conn = db::open()?;
            let videos = lookup::by_thumbs(&conn, &thumbs)?;
            for source in manifest::sources(&videos)? {
                println!("{source}");
            }
            Ok(())
        }
        Commands::Manifest { thumbs } => {
            let conn = db::open()?;
            let videos = lookup::by_thumbs(&conn, &thumbs)?;
            println!("{}", manifest::render(&videos)?);
            Ok(())
        }
        Commands::Progress {
            id,
            pos,
            duration,
            watched,
        } => {
            let conn = db::open()?;
            db::set_progress(&conn, id, pos, duration, watched)?;
            Ok(())
        }
        Commands::Copy { both, thumbs } => {
            let conn = db::open()?;
            let videos = lookup::by_thumbs(&conn, &thumbs)?;
            println!("{}", copy::copy(&videos, both)?);
            Ok(())
        }
        Commands::Placeholder => {
            println!("{}", thumbs::ensure_placeholder()?.display());
            Ok(())
        }
        Commands::Thumbs { command } => {
            let conn = db::open()?;
            match command {
                ThumbsCommands::Ensure {
                    method,
                    thumbs: selected,
                    category,
                    dir,
                } => {
                    let method = thumbs::ThumbMethod::parse(&method)?;
                    let videos = videos_for_thumbs(&conn, &selected, category, dir)?;
                    println!("{}", thumbs::ensure_many(&conn, &videos, method));
                }
                ThumbsCommands::Regenerate {
                    method,
                    thumbs: selected,
                    category,
                    dir,
                } => {
                    let method = thumbs::ThumbMethod::parse(&method)?;
                    let videos = videos_for_thumbs(&conn, &selected, category, dir)?;
                    println!("{}", thumbs::regenerate_many(&conn, &videos, method));
                }
            }
            Ok(())
        }
        Commands::Categories { no_system } => {
            let conn = db::open()?;
            for path in catalog::categories(&conn, !no_system)? {
                println!("{path}");
            }
            Ok(())
        }
        Commands::Dirs => {
            let conn = db::open()?;
            for dir in catalog::dirs(&conn)? {
                println!("{}", dir.display());
            }
            Ok(())
        }
        Commands::Assign {
            category,
            create,
            thumbs,
        } => {
            let conn = db::open()?;
            println!("{}", catalog::assign(&conn, &thumbs, &category, create)?);
            Ok(())
        }
        Commands::Delete { files, thumbs } => {
            let conn = db::open()?;
            println!("{}", catalog::delete(&conn, &thumbs, files)?);
            Ok(())
        }
        Commands::Download {
            format,
            dir,
            thumbs,
        } => {
            let conn = db::open()?;
            let dir = if dir.is_absolute() {
                dir
            } else {
                std::env::current_dir()
                    .context("no se pudo obtener el directorio actual")?
                    .join(dir)
            };
            println!("{}", download::enqueue(&conn, &thumbs, &format, &dir)?);
            Ok(())
        }
        Commands::DownloadCandidates { thumbs } => {
            let conn = db::open()?;
            for video in download::candidates(&conn, &thumbs)? {
                println!("{}", video.id);
            }
            Ok(())
        }
        Commands::DownloadQualities => {
            for (label, format) in download::qualities() {
                println!("{label}\t{format}");
            }
            Ok(())
        }
        Commands::Category { command } => {
            let conn = db::open()?;
            match command {
                CategoryCommands::Create { path } => {
                    println!("{}", catalog::create_category(&conn, &path)?);
                }
                CategoryCommands::Rename { path, name } => {
                    println!("{}", catalog::rename_category(&conn, &path, &name)?);
                }
                CategoryCommands::Move { path, parent } => {
                    println!(
                        "{}",
                        catalog::move_category(&conn, &path, parent.as_deref())?
                    );
                }
                CategoryCommands::Delete { path } => {
                    println!("{}", catalog::delete_category(&conn, &path)?);
                }
            }
            Ok(())
        }
        Commands::DownloadWorker { ids, format, dir } => {
            let conn = db::open()?;
            let dir = if dir.is_absolute() {
                dir
            } else {
                std::env::current_dir()
                    .context("no se pudo obtener el directorio actual")?
                    .join(dir)
            };
            fs::create_dir_all(&dir)?;
            download::worker(&conn, &ids, &format, &dir)
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}
