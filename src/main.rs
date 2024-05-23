//! The `kcl` lsp server.

#![deny(missing_docs)]

use std::sync::Arc;

use anyhow::{bail, Result};
use clap::Parser;
use slog::Drain;
use tower_lsp::{LspService, Server as LspServer};
use tracing_subscriber::{prelude::*, Layer};

lazy_static::lazy_static! {
/// Initialize the logger.
    // We need a slog::Logger for steno and when we export out the logs from re-exec-ed processes.
    pub static ref LOGGER: slog::Logger = {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        slog::Logger::root(drain, slog::slog_o!())
    };
}

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields.
#[derive(Parser, Debug, Clone)]
#[clap(version = clap::crate_version!(), author = clap::crate_authors!("\n"))]
pub struct Opts {
    /// Print debug info
    #[clap(short, long)]
    pub debug: bool,

    /// Print logs as json
    #[clap(short, long)]
    pub json: bool,

    /// The subcommand to run.
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

impl Opts {
    /// Setup our logger.
    pub fn create_logger(&self) -> slog::Logger {
        if self.json {
            let drain = slog_json::Json::default(std::io::stderr()).fuse();
            self.async_root_logger(drain)
        } else {
            let decorator = slog_term::TermDecorator::new().build();
            let drain = slog_term::FullFormat::new(decorator).build().fuse();
            self.async_root_logger(drain)
        }
    }

    fn async_root_logger<T>(&self, drain: T) -> slog::Logger
    where
        T: slog::Drain + Send + 'static,
        <T as slog::Drain>::Err: std::fmt::Debug,
    {
        let level = if self.debug {
            slog::Level::Debug
        } else {
            slog::Level::Info
        };

        let level_drain = slog::LevelFilter(drain, level).fuse();
        let async_drain = slog_async::Async::new(level_drain).build().fuse();
        slog::Logger::root(async_drain, slog::o!())
    }
}

/// A subcommand for our cli.
#[derive(Parser, Debug, Clone)]
pub enum SubCommand {
    /// Run the server.
    Server(kcl_lib::lsp::kcl::Server),
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let level_filter = if opts.debug {
        tracing_subscriber::filter::LevelFilter::DEBUG
    } else {
        tracing_subscriber::filter::LevelFilter::INFO
    };

    // Format fields using the provided closure.
    // We want to make this very consise otherwise the logs are not able to be read by humans.
    let format = tracing_subscriber::fmt::format::debug_fn(|writer, field, value| {
        if format!("{}", field) == "message" {
            write!(writer, "{}: {:?}", field, value)
        } else {
            write!(writer, "{}", field)
        }
    })
    // Separate each field with a comma.
    // This method is provided by an extension trait in the
    // `tracing-subscriber` prelude.
    .delimited(", ");

    let (json, plain) = if opts.json {
        // Cloud run likes json formatted logs if possible.
        // See: https://cloud.google.com/run/docs/logging
        // We could probably format these specifically for cloud run if we wanted,
        // will save that as a TODO: https://cloud.google.com/run/docs/logging#special-fields
        (
            Some(tracing_subscriber::fmt::layer().json().with_filter(level_filter)),
            None,
        )
    } else {
        (
            None,
            Some(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .fmt_fields(format)
                    .with_filter(level_filter),
            ),
        )
    };

    // Initialize the tracing.
    tracing_subscriber::registry().with(json).with(plain).init();

    if let Err(err) = run_cmd(&opts).await {
        bail!("running cmd `{:?}` failed: {:?}", &opts.subcmd, err);
    }

    Ok(())
}

async fn run_cmd(opts: &Opts) -> Result<()> {
    match &opts.subcmd {
        SubCommand::Server(s) => {
            let stdlib = kcl_lib::std::StdLib::new();
            let stdlib_completions = kcl_lib::lsp::kcl::get_completions_from_stdlib(&stdlib)?;
            let stdlib_signatures = kcl_lib::lsp::kcl::get_signatures_from_stdlib(&stdlib)?;
            let fs = Arc::new(kcl_lib::fs::FileManager::new());
            // We can unwrap here because we know the tokeniser is valid, since
            // we have a test for it.
            let token_types = kcl_lib::token::TokenType::all_semantic_token_types().unwrap();

            let (service, socket) = LspService::new(|client| kcl_lib::lsp::kcl::Backend {
                client,
                stdlib_completions,
                stdlib_signatures,
                token_types,
                fs,
                token_map: Default::default(),
                ast_map: Default::default(),
                code_map: Default::default(),
                memory_map: Default::default(),
                diagnostics_map: Default::default(),
                symbols_map: Default::default(),
                semantic_tokens_map: Default::default(),
                workspace_folders: Default::default(),
                can_send_telemetry: false,
                zoo_client: kittycad::Client::new(""),
                current_handle: Default::default(),
                can_execute: Default::default(),
                executor_ctx: Default::default(),
                is_initialized: Default::default(),
            });

            // TODO find a way to ctrl+c on windows.
            #[cfg(not(target_os = "windows"))]
            {
                // For Cloud run & ctrl+c, shutdown gracefully.
                // "The main process inside the container will receive SIGTERM, and after a grace period,
                // SIGKILL."
                // Registering SIGKILL here will panic at runtime, so let's avoid that.
                use signal_hook::{
                    consts::{SIGINT, SIGTERM},
                    iterator::Signals,
                };
                let mut signals = Signals::new([SIGINT, SIGTERM])?;

                tokio::spawn(async move {
                    if let Some(sig) = signals.forever().next() {
                        log::info!("received signal: {:?}", sig);
                        log::info!("triggering cleanup...");

                        // Exit the process.
                        log::info!("all clean, exiting!");
                        std::process::exit(0);
                    }
                });
            }

            if s.stdio {
                // Listen on stdin and stdout.
                let stdin = tokio::io::stdin();
                let stdout = tokio::io::stdout();
                LspServer::new(stdin, stdout, socket).serve(service).await;
            } else {
                // Listen on a tcp stream.
                let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", s.socket)).await?;
                let (stream, _) = listener.accept().await?;
                let (read, write) = tokio::io::split(stream);
                LspServer::new(read, write, socket).serve(service).await;
            }
        }
    }

    Ok(())
}
