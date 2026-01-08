use crate::dist::{DistClient, DistOptions};
use crate::oci_components::default_cache_root;
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "greentic-dist")]
#[command(about = "Greentic component resolver and cache manager")]
pub struct Cli {
    /// Override cache directory
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,
    /// Offline mode (disable network fetches)
    #[arg(long, global = true)]
    pub offline: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Resolve a reference and print its digest
    Resolve {
        reference: String,
        #[arg(long)]
        json: bool,
    },
    /// Pull a reference or lockfile into the cache
    Pull {
        reference: Option<String>,
        #[arg(long)]
        lock: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Cache management commands
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    /// Authentication commands (stub)
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheCommand {
    /// List cached digests
    Ls {
        #[arg(long)]
        json: bool,
    },
    /// Remove cached digests
    Rm {
        digests: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Garbage-collect broken cache entries
    Gc {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
    /// Stub login hook for future repo/store auth
    Login { target: String },
}

#[derive(Serialize)]
struct ResolveOutput<'a> {
    reference: &'a str,
    digest: &'a str,
}

#[derive(Serialize)]
struct PullOutput<'a> {
    reference: &'a str,
    digest: &'a str,
    cache_path: Option<&'a std::path::Path>,
    fetched: bool,
}

pub async fn run_from_env() -> Result<(), CliError> {
    let cli = Cli::parse();
    run(cli).await
}

pub struct CliError {
    pub code: i32,
    pub message: String,
}

pub async fn run(cli: Cli) -> Result<(), CliError> {
    let mut opts = DistOptions::default();
    if let Some(dir) = cli.cache_dir {
        opts.cache_dir = dir;
    } else {
        opts.cache_dir = default_cache_root();
    }
    opts.offline = cli.offline || opts.offline;

    let client = DistClient::new(opts);

    match cli.command {
        Commands::Resolve { reference, json } => {
            let resolved = client
                .resolve_ref(&reference)
                .await
                .map_err(CliError::from_dist)?;
            if json {
                let out = ResolveOutput {
                    reference: &reference,
                    digest: &resolved.digest,
                };
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
            } else {
                println!("{}", resolved.digest);
            }
        }
        Commands::Pull {
            reference,
            lock,
            json,
        } => {
            if let Some(lock_path) = lock {
                let resolved = client
                    .pull_lock(&lock_path)
                    .await
                    .map_err(CliError::from_dist)?;
                if json {
                    let payload: Vec<_> = resolved
                        .iter()
                        .map(|r| PullOutput {
                            reference: "",
                            digest: &r.digest,
                            cache_path: r.cache_path.as_deref(),
                            fetched: r.fetched,
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
                } else {
                    for r in resolved {
                        let path = r
                            .cache_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        println!("{} {}", r.digest, path);
                    }
                }
            } else if let Some(reference) = reference {
                let resolved = client
                    .ensure_cached(&reference)
                    .await
                    .map_err(CliError::from_dist)?;
                if json {
                    let out = PullOutput {
                        reference: &reference,
                        digest: &resolved.digest,
                        cache_path: resolved.cache_path.as_deref(),
                        fetched: resolved.fetched,
                    };
                    println!("{}", serde_json::to_string_pretty(&out).unwrap());
                } else if let Some(path) = &resolved.cache_path {
                    println!("{}", path.display());
                } else {
                    println!("{}", resolved.digest);
                }
            } else {
                return Err(CliError {
                    code: 2,
                    message: "pull requires either a reference or --lock".into(),
                });
            }
        }
        Commands::Cache { command } => match command {
            CacheCommand::Ls { json } => {
                let entries = client.list_cache();
                if json {
                    println!("{}", serde_json::to_string_pretty(&entries).unwrap());
                } else {
                    for digest in entries {
                        println!("{digest}");
                    }
                }
            }
            CacheCommand::Rm { digests, json } => {
                client
                    .remove_cached(&digests)
                    .map_err(CliError::from_dist)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&digests).unwrap());
                }
            }
            CacheCommand::Gc { json } => {
                let removed = client.gc().map_err(CliError::from_dist)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&removed).unwrap());
                } else if !removed.is_empty() {
                    eprintln!("removed {}", removed.join(", "));
                }
            }
        },
        Commands::Auth { command } => match command {
            AuthCommand::Login { target } => {
                return Err(CliError {
                    code: 5,
                    message: format!(
                        "auth login for `{target}` is not implemented yet; stubbed for future store/repo"
                    ),
                });
            }
        },
    }

    Ok(())
}

impl CliError {
    pub fn from_dist(err: crate::dist::DistError) -> Self {
        Self {
            code: err.exit_code(),
            message: err.to_string(),
        }
    }
}
