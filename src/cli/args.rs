//! Superfície da CLI (só definição): a árvore de comandos do clap.
//!
//! O quê: flags e subcomandos de `schematize-connectors`.
//! Onde: parseada por `cli::executar`; o golden `tests/superficie-cli.txt` trava a forma.

use clap::{Parser, Subcommand, ValueEnum};

/// Idioma pedido por `--lang`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum LangArg {
    /// Português.
    Pt,
    /// English.
    En,
}

/// Cliente MCP alvo do `enable`/`disable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Cliente {
    /// Claude Code (CLI `claude`).
    #[value(name = "claude-code")]
    ClaudeCode,
    /// Cursor (`~/.cursor/mcp.json`).
    #[value(name = "cursor")]
    Cursor,
    /// VS Code (`.vscode/mcp.json`).
    #[value(name = "vscode")]
    Vscode,
}

/// Slot de anúncio consultável em `anuncios`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SlotArg {
    /// Tela inicial do app.
    #[value(name = "tela-inicial")]
    TelaInicial,
    /// Estado vazio.
    #[value(name = "estado-vazio")]
    EstadoVazio,
    /// Banner da CLI.
    #[value(name = "banner-cli")]
    BannerCli,
}

/// `schematize-connectors`: catálogo de conectores MCP com prévia.
#[derive(Parser, Debug)]
#[command(
    name = "schematize-connectors",
    version,
    about = "schematize connectors: MCP connector catalog with a read-only preview",
    long_about = "Lists MCP connectors, previews their tools (read-only) and prints the exact \
                  command to enable one. Nothing is changed without --aplicar."
)]
pub struct Cli {
    /// Output language (default: from the environment).
    #[arg(long, global = true, value_enum)]
    pub lang: Option<LangArg>,
    /// Never show the house ad banner.
    #[arg(long, global = true)]
    pub sem_anuncios: bool,
    /// Subcomando.
    #[command(subcommand)]
    pub cmd: Cmd,
}

/// Subcomandos.
#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// List the connectors (main one first) with their status.
    List {
        /// Machine-readable output. Stable keys, never translated.
        #[arg(long)]
        json: bool,
    },
    /// Show a connector's tools. Offline by default; --ao-vivo asks the server (read-only).
    Preview {
        /// Connector id (see `list`).
        id: String,
        /// Ask the server (`tools/list`) instead of the embedded list.
        #[arg(long)]
        ao_vivo: bool,
        /// Machine-readable output. Stable keys, never translated.
        #[arg(long)]
        json: bool,
    },
    /// Print the exact command/snippet to enable a connector. --aplicar runs it (claude-code only).
    Enable {
        /// Connector id.
        id: String,
        /// MCP client.
        #[arg(long, value_enum, default_value = "claude-code")]
        cliente: Cliente,
        /// Run `claude mcp add` for you (claude-code only; needs the key in the env var).
        #[arg(long)]
        aplicar: bool,
        /// Machine-readable output. Stable keys, never translated.
        #[arg(long)]
        json: bool,
    },
    /// Print the command to remove a connector. --aplicar runs it (claude-code only).
    Disable {
        /// Connector id.
        id: String,
        /// MCP client.
        #[arg(long, value_enum, default_value = "claude-code")]
        cliente: Cliente,
        /// Run `claude mcp remove` for you (claude-code only).
        #[arg(long)]
        aplicar: bool,
        /// Machine-readable output. Stable keys, never translated.
        #[arg(long)]
        json: bool,
    },
    /// Audit what ad the current context would show, and why not (informational only).
    Anuncios {
        /// Slot to evaluate.
        #[arg(long, value_enum, default_value = "banner-cli")]
        slot: SlotArg,
        /// Machine-readable output. Stable keys, never translated.
        #[arg(long)]
        json: bool,
    },
    /// Catalog sizes/order, validation of every connector, opt-out and tty state.
    Diag {
        /// Machine-readable output. Stable keys, never translated.
        #[arg(long)]
        json: bool,
    },
}
