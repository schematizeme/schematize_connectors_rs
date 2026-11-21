//! Falhas da CLI e o mapa para exit codes.
//!
//! O quê: `Falha` carrega exit code, slug estável (vai no `--json`) e a mensagem humana.
//! Onde: todo comando devolve `Result<(), Falha>`; `cli::executar` imprime e converte em exit.
//!
//! Exit codes: 0 ok, 1 uso/pedido recusado, 2 argumento inválido (clap), 3 sem endpoint/MCP,
//! 4 exige segredo, 5 rede/protocolo, 6 timeout.

use crate::anuncio::Idioma;
use crate::cli::texto::{tf, K};
use crate::mcp::ErroMcp;

/// Uma falha de comando.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Falha {
    /// Exit code do processo.
    pub exit: i32,
    /// Identificador estável, máquina-legível (nunca traduzido).
    pub slug: &'static str,
    /// Mensagem humana já traduzida, acionável e sem culpa.
    pub msg: String,
    /// Nome (nunca valor) da env var envolvida, se houver.
    pub env_var: Option<String>,
}

impl Falha {
    /// O quê: falha de uso/pedido recusado (exit 1). Onde: id inválido, `--aplicar` recusado.
    pub fn uso(slug: &'static str, msg: String) -> Self {
        Self { exit: 1, slug, msg, env_var: None }
    }

    /// O quê: conector sem endpoint/MCP (exit 3). Onde: preview/enable de pendente/sem MCP.
    pub fn sem_endpoint(msg: String) -> Self {
        Self { exit: 3, slug: "sem_endpoint", msg, env_var: None }
    }

    /// O quê: falta a chave (exit 4), citando só o NOME da variável. Onde: preview/enable.
    pub fn exige_segredo(msg: String, env_var: &str) -> Self {
        Self { exit: 4, slug: "exige_segredo", msg, env_var: Some(env_var.to_string()) }
    }

    /// O quê: traduz um `ErroMcp` da prévia em falha localizada com o exit certo.
    /// Onde: `preview --ao-vivo`. `env_var` é a variável do conector (ou vazia).
    pub fn de_mcp(e: &ErroMcp, l: Idioma, env_var: &str, link: &str, prefixo: &str) -> Self {
        let (exit, slug, msg) = match e {
            ErroMcp::SemEndpoint => (3, "sem_endpoint", tf(l, K::ErroSemEndpoint, &[])),
            ErroMcp::ExigeSegredo { env_var: v } => {
                (4, "exige_segredo", tf(l, K::ExigeChave, &[v, link, prefixo]))
            }
            ErroMcp::Rede(m) => (5, "rede", tf(l, K::ErroRede, &[m])),
            ErroMcp::Protocolo(m) => (5, "protocolo", tf(l, K::ErroProtocolo, &[m])),
            ErroMcp::RespostaGrande => (5, "resposta_grande", tf(l, K::ErroGrande, &[])),
            ErroMcp::Timeout => (6, "timeout", tf(l, K::ErroTimeout, &[])),
            ErroMcp::Recusado(m) => (5, "recusado", tf(l, K::ErroRecusado, &[m, env_var])),
        };
        let nome = match e {
            ErroMcp::ExigeSegredo { env_var: v } => Some(v.clone()),
            _ => None,
        };
        Self { exit, slug, msg, env_var: nome }
    }
}

impl From<std::io::Error> for Falha {
    /// O quê: erro de escrita na saída; pipe fechado (`| head`) é saída normal (exit 0).
    /// Onde: qualquer `writeln!` com `?`.
    fn from(e: std::io::Error) -> Self {
        let fechado = e.kind() == std::io::ErrorKind::BrokenPipe;
        let slug = if fechado { "saida_fechada" } else { "io" };
        Self { exit: i32::from(!fechado), slug, msg: e.to_string(), env_var: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapeia_cada_erro_mcp_para_o_exit_do_contrato() {
        let casos = [
            (ErroMcp::SemEndpoint, 3),
            (ErroMcp::ExigeSegredo { env_var: "V".into() }, 4),
            (ErroMcp::Rede("x".into()), 5),
            (ErroMcp::Protocolo("x".into()), 5),
            (ErroMcp::RespostaGrande, 5),
            (ErroMcp::Recusado("x".into()), 5),
            (ErroMcp::Timeout, 6),
        ];
        for (e, exit) in casos {
            for l in [Idioma::Pt, Idioma::En] {
                let f = Falha::de_mcp(&e, l, "V", "https://x", "sk_");
                assert_eq!(f.exit, exit, "{e:?}");
                assert!(!f.msg.is_empty() && !f.msg.contains('{'));
            }
        }
    }

    #[test]
    fn pipe_fechado_nao_e_erro() {
        let f = Falha::from(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        assert_eq!((f.exit, f.slug), (0, "saida_fechada"));
        let g = Falha::from(std::io::Error::other("x"));
        assert_eq!(g.exit, 1);
    }
}
