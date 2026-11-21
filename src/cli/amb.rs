//! Ambiente injetável da CLI: env, tty, relógio, previadores e o executor do `claude`.
//!
//! O quê: tudo que a CLI toca do mundo vem por aqui, para testar com fakes.
//! Onde: `main.rs` monta o real (`Amb::real`); os testes montam o seu.

use crate::anuncio::{Data, Idioma};
use crate::cli::claude::{Claude, ClaudeReal};
use crate::mcp::{PreviaEstatica, PreviaViva, Previador};
use std::io::IsTerminal;

/// Leitor de variável de ambiente (injetável).
pub type LeEnv<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Dependências externas da CLI.
pub struct Amb<'a> {
    /// Leitor de env.
    pub env: LeEnv<'a>,
    /// `true` se o stdout é um terminal interativo.
    pub tty: bool,
    /// "Hoje" (UTC).
    pub hoje: Data,
    /// Prévia offline.
    pub estatico: &'a dyn Previador,
    /// Prévia ao vivo.
    pub vivo: &'a dyn Previador,
    /// Executor do `claude`.
    pub claude: &'a dyn Claude,
}

/// Instâncias reais, para o `main` emprestar a `Amb`.
pub struct Reais {
    /// Prévia offline.
    pub estatico: PreviaEstatica,
    /// Prévia ao vivo.
    pub vivo: PreviaViva,
    /// Executor do `claude`.
    pub claude: ClaudeReal,
}

impl Reais {
    /// O quê: cria os componentes reais. Onde: `main`.
    pub fn novo() -> Self {
        Self { estatico: PreviaEstatica, vivo: PreviaViva::default(), claude: ClaudeReal }
    }
}

/// O quê: lê `std::env` como `Option<String>` (não-Unicode vira `None`). Onde: `main`.
pub fn env_real(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

/// O quê: `true` se o stdout é terminal. Onde: `main`.
pub fn stdout_e_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// O quê: idioma pelo ambiente (`SCHEMATIZE_LANG`, `LC_ALL`, `LC_MESSAGES`, `LANG`); `pt*` é
/// português, qualquer outra coisa (ou nada) é inglês. Onde: `executar`, quando não há `--lang`.
pub fn idioma_de_env(get: LeEnv<'_>) -> Idioma {
    for k in ["SCHEMATIZE_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(v) = get(k).filter(|v| !v.trim().is_empty()) {
            return if v.trim().to_lowercase().starts_with("pt") { Idioma::Pt } else { Idioma::En };
        }
    }
    Idioma::En
}

/// O quê: data civil UTC de hoje (relógio do sistema; relógio quebrado vira 1970-01-01).
/// Onde: `main`, para o filtro de expiração dos anúncios.
pub fn hoje_utc() -> Data {
    let s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    data_de_dias((s / 86_400) as i64)
}

/// O quê: converte dias desde 1970-01-01 em data civil (algoritmo de Hinnant).
/// Onde: `hoje_utc` e testes.
pub fn data_de_dias(dias: i64) -> Data {
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let ano = (yoe + era * 400 + i64::from(mes <= 2)) as u16;
    Data { ano, mes, dia }
}

/// O quê: o arquivo `<config>/schematize/connectors/sem-anuncios` existe? (config = `XDG_CONFIG_HOME`
/// ou `$HOME/.config`). Onde: opt-out por config, em `executar`.
pub fn optout_de_config(get: LeEnv<'_>) -> bool {
    let base = match get("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(x) => std::path::PathBuf::from(x),
        None => match get("HOME").filter(|v| !v.is_empty()) {
            Some(h) => std::path::Path::new(&h).join(".config"),
            None => return false,
        },
    };
    base.join("schematize").join("connectors").join("sem-anuncios").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datas_conhecidas() {
        assert_eq!(data_de_dias(0), Data { ano: 1970, mes: 1, dia: 1 });
        assert_eq!(data_de_dias(20_453), Data { ano: 2025, mes: 12, dia: 31 });
        assert_eq!(data_de_dias(19_782), Data { ano: 2024, mes: 2, dia: 29 });
    }

    #[test]
    fn idioma_segue_a_ordem_do_env() {
        let pt = |k: &str| (k == "LANG").then(|| "pt_BR.UTF-8".to_string());
        assert_eq!(idioma_de_env(&pt), Idioma::Pt);
        let en = |k: &str| (k == "LANG").then(|| "en_US.UTF-8".to_string());
        assert_eq!(idioma_de_env(&en), Idioma::En);
        let sobre = |k: &str| match k {
            "SCHEMATIZE_LANG" => Some("en".into()),
            "LANG" => Some("pt_BR".into()),
            _ => None,
        };
        assert_eq!(idioma_de_env(&sobre), Idioma::En);
        assert_eq!(idioma_de_env(&|_: &str| None), Idioma::En);
    }
}
