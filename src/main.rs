//! `schematize-connectors`: binário da CLI. Toda a lógica vive em `connectors::cli`.
//!
//! O quê: monta o ambiente real e delega a `cli::executar`.
//! Onde: ponto de entrada do processo; é o único lugar que chama `process::exit`.

use connectors::cli::amb::{env_real, hoje_utc, stdout_e_tty, Amb, Reais};
use connectors::cli::executar;

/// O quê: executa a CLI com os argumentos do processo e sai com o exit code dela.
/// Onde: `schematize-connectors <comando>`. Argumento não-UTF-8 é decodificado com perda
/// (o clap então recusa por uso inválido, exit 2) — nunca panic.
fn main() {
    let args: Vec<String> = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
    let reais = Reais::novo();
    let amb = Amb {
        env: &env_real,
        tty: stdout_e_tty(),
        hoje: hoje_utc(),
        estatico: &reais.estatico,
        vivo: &reais.vivo,
        claude: &reais.claude,
    };
    let code = executar(&args, &amb, &mut std::io::stdout(), &mut std::io::stderr());
    std::process::exit(code);
}
