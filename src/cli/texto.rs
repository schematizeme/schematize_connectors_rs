//! i18n da saída humana (pt/en). O `--json` NUNCA passa por aqui: não é traduzido.
//!
//! O quê: catálogo `K` -> (pt, en) com marcadores `{0}`, `{1}`... e `tf` que os preenche.
//! Onde: toda mensagem da CLI. `match` exaustivo: chave sem tradução não compila.
//! Tom (piso 48 da casa): acionável e sem culpa — "faço X por você" em vez de "você errou".

use crate::anuncio::Idioma;

/// Chaves de mensagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum K {
    ColNome,
    ColDominio,
    ColStatus,
    StAtivo,
    StPendente,
    StSemMcp,
    ListaRodape,
    IdVazio,
    IdDesconhecido,
    SemEndpointPendente,
    SemEndpointSemMcp,
    ExigeChave,
    ErroRede,
    ErroProtocolo,
    ErroGrande,
    ErroTimeout,
    ErroRecusado,
    ErroSemEndpoint,
    PreviaTitulo,
    NotaEstatica,
    NotaViva,
    Leitura,
    Escrita,
    AvisoEscrita,
    ChaveDefinida,
    ChaveFormato,
    EnClaude,
    EnCursor,
    EnVscode,
    EnNada,
    EnEnv,
    EnOauth,
    EnAplicarSo,
    EnSemClaude,
    EnAviso,
    EnOk,
    EnFalhou,
    DisClaude,
    DisManual,
    DisOk,
    AnContexto,
    AnBloqJson,
    AnBloqTty,
    AnBloqOptout,
    AnNenhum,
    AnEntregue,
    AnCandidato,
    DgCatalogo,
    DgOrdem,
    DgConector,
    DgOptout,
    DgTty,
    DgAnuncios,
    Sim,
    Nao,
    Ok,
    Falha,
    OptEnv,
    OptFlag,
    OptConfig,
    Nenhum,
}

/// O quê: textos (pt, en) das chaves de lista, erro e prévia. Onde: `par`.
fn par_a(k: K) -> Option<(&'static str, &'static str)> {
    Some(match k {
        K::ColNome => ("NOME", "NAME"),
        K::ColDominio => ("DOMÍNIO", "DOMAIN"),
        K::ColStatus => ("STATUS", "STATUS"),
        K::StAtivo => ("ativo", "active"),
        K::StPendente => ("endpoint pendente", "endpoint pending"),
        K::StSemMcp => ("sem MCP", "no MCP"),
        K::ListaRodape => (
            "★ = conector principal da casa. Veja as ferramentas com: schematize-connectors preview <id>",
            "★ = the house's main connector. See the tools with: schematize-connectors preview <id>",
        ),
        K::IdVazio => (
            "informe o id de um conector (veja os ids com: schematize-connectors list)",
            "give a connector id (see the ids with: schematize-connectors list)",
        ),
        K::IdDesconhecido => (
            "não conheço o conector \"{0}\". Os disponíveis são: {1}",
            "I don't know the connector \"{0}\". The available ones are: {1}",
        ),
        K::SemEndpointPendente => (
            "o endpoint MCP de {0} ainda não foi cadastrado; adicione-o em src/catalogo.rs (campo endpoint, https do domínio {1}) e marque o status como ativo",
            "the MCP endpoint of {0} has not been registered yet; add it in src/catalogo.rs (endpoint field, https on the domain {1}) and set the status to active",
        ),
        K::SemEndpointSemMcp => (
            "{0} ainda não tem servidor MCP; quando tiver, cadastre o endpoint em src/catalogo.rs",
            "{0} has no MCP server yet; once it does, register its endpoint in src/catalogo.rs",
        ),
        K::ExigeChave => (
            "defina {0} com uma chave {2} só de leitura (crie em {1}) e rode de novo",
            "set {0} to a read-only {2} key (create one at {1}) and run again",
        ),
        _ => return None,
    })
}

/// O quê: textos das chaves de erro de rede e prévia. Onde: `par`.
fn par_b(k: K) -> Option<(&'static str, &'static str)> {
    Some(match k {
        K::ErroRede => (
            "não consegui falar com o servidor ({0}); verifique a conexão e tente de novo",
            "I could not reach the server ({0}); check the connection and try again",
        ),
        K::ErroProtocolo => (
            "o servidor respondeu algo inesperado ({0}); use a lista embutida (sem --ao-vivo)",
            "the server answered something unexpected ({0}); use the embedded list (without --ao-vivo)",
        ),
        K::ErroGrande => (
            "a resposta do servidor passou do limite seguro e foi descartada; use a lista embutida (sem --ao-vivo)",
            "the server's answer went over the safe limit and was dropped; use the embedded list (without --ao-vivo)",
        ),
        K::ErroTimeout => (
            "o servidor demorou demais para responder; tente de novo em instantes",
            "the server took too long to answer; try again in a moment",
        ),
        K::ErroRecusado => (
            "o servidor recusou o pedido ({0}); confira a chave guardada em {1}",
            "the server refused the request ({0}); check the key stored in {1}",
        ),
        K::ErroSemEndpoint => (
            "este conector não tem endpoint MCP para consultar; veja a lista embutida (sem --ao-vivo)",
            "this connector has no MCP endpoint to query; see the embedded list (without --ao-vivo)",
        ),
        K::PreviaTitulo => ("{0}: {1} ferramentas", "{0}: {1} tools"),
        K::NotaEstatica => (
            "Lista EMBUTIDA (offline), pode estar defasada. Use --ao-vivo para consultar o servidor.",
            "EMBEDDED list (offline), may be outdated. Use --ao-vivo to query the server.",
        ),
        K::NotaViva => ("Consultado ao vivo no servidor {0}.", "Queried live on the server {0}."),
        K::Leitura => ("leitura", "read"),
        K::Escrita => ("ESCRITA", "WRITE"),
        K::AvisoEscrita => (
            "Ferramentas de escrita exigem uma chave sk_*; o login OAuth é somente leitura. A prévia nunca executa ferramenta alguma.",
            "Write tools need an sk_* key; the OAuth login is read-only. The preview never runs any tool.",
        ),
        K::ChaveDefinida => (
            "Chave: a variável {0} está definida (o valor nunca é exibido).",
            "Key: the variable {0} is set (the value is never shown).",
        ),
        K::ChaveFormato => (
            "Atenção: o valor de {0} não começa com {1}; confira se é a chave certa.",
            "Heads-up: the value of {0} does not start with {1}; check it is the right key.",
        ),
        _ => return None,
    })
}

/// O quê: textos de enable/disable. Onde: `par`.
fn par_c(k: K) -> Option<(&'static str, &'static str)> {
    Some(match k {
        K::EnClaude => ("Para ativar {0} no Claude Code, rode:", "To enable {0} in Claude Code, run:"),
        K::EnCursor => ("Para ativar {0} no Cursor, acrescente em {1}:", "To enable {0} in Cursor, add to {1}:"),
        K::EnVscode => ("Para ativar {0} no VS Code, acrescente em {1}:", "To enable {0} in VS Code, add to {1}:"),
        K::EnNada => (
            "Nada foi alterado. Use --aplicar para eu rodar o comando por você (só Claude Code).",
            "Nothing was changed. Use --aplicar and I will run the command for you (Claude Code only).",
        ),
        K::EnEnv => (
            "A chave fica na variável {0}: quem a expande é o seu shell/cliente, ela não aparece aqui.",
            "The key lives in the variable {0}: your shell/client expands it, it is not shown here.",
        ),
        K::EnOauth => (
            "Este conector usa OAuth: o cliente abre o login na primeira conexão (somente leitura).",
            "This connector uses OAuth: the client opens the login on first connect (read-only).",
        ),
        K::EnAplicarSo => (
            "--aplicar só existe para --cliente claude-code; para os outros, cole o trecho impresso (sem --aplicar).",
            "--aplicar only exists for --cliente claude-code; for the others, paste the printed snippet (without --aplicar).",
        ),
        K::EnSemClaude => (
            "não encontrei o comando `claude` no PATH. Instale o Claude Code ou rode você mesmo o comando impresso por: schematize-connectors enable {0}",
            "I could not find the `claude` command in PATH. Install Claude Code or run the printed command yourself via: schematize-connectors enable {0}",
        ),
        K::EnAviso => (
            "Atenção: o Claude Code guardará a chave na configuração dele, e ela fica visível na lista de processos por instantes.",
            "Heads-up: Claude Code will store the key in its own config, and it is visible in the process list for a moment.",
        ),
        K::EnOk => ("Feito: {0} adicionado ao Claude Code.", "Done: {0} added to Claude Code."),
        K::EnFalhou => (
            "o `claude` terminou com erro (código {0}); a saída dele está acima. Nada mais foi alterado por mim.",
            "`claude` ended with an error (code {0}); its output is above. I changed nothing else.",
        ),
        K::DisClaude => ("Para remover {0} do Claude Code, rode:", "To remove {0} from Claude Code, run:"),
        K::DisManual => (
            "Para remover {0}, apague a entrada \"{0}\" de {1}.",
            "To remove {0}, delete the \"{0}\" entry from {1}.",
        ),
        K::DisOk => ("Feito: {0} removido do Claude Code.", "Done: {0} removed from Claude Code."),
        _ => return None,
    })
}

/// O quê: textos de anúncios, diag e rótulos soltos. Onde: `par`.
fn par_d(k: K) -> Option<(&'static str, &'static str)> {
    Some(match k {
        K::AnContexto => (
            "Contexto: slot={0}, terminal interativo={1}, --json={2}, opt-out={3}",
            "Context: slot={0}, interactive terminal={1}, --json={2}, opt-out={3}",
        ),
        K::AnBloqJson => (
            "bloqueado: saída em JSON (máquina/agente nunca recebe anúncio)",
            "blocked: JSON output (machines/agents never get ads)",
        ),
        K::AnBloqTty => (
            "bloqueado: a saída não é um terminal interativo",
            "blocked: the output is not an interactive terminal",
        ),
        K::AnBloqOptout => ("bloqueado: opt-out ativo ({0})", "blocked: opt-out is on ({0})"),
        K::AnNenhum => (
            "nenhum anúncio elegível neste slot (expirados, conector já ativo ou fora do slot)",
            "no eligible ad for this slot (expired, connector already active or outside the slot)",
        ),
        K::AnEntregue => ("Exibiria agora:", "Would show now:"),
        K::AnCandidato => ("Escolheria sem o bloqueio: {0}", "Would pick without the block: {0}"),
        K::DgCatalogo => {
            ("Catálogo: {0} conectores, principal: {1}", "Catalog: {0} connectors, main: {1}")
        }
        K::DgOrdem => ("Ordem: {0} ({1})", "Order: {0} ({1})"),
        K::DgConector => ("  {0}: {1}", "  {0}: {1}"),
        K::DgOptout => ("Opt-out de anúncios: {0}", "Ad opt-out: {0}"),
        K::DgTty => ("Terminal interativo: {0}", "Interactive terminal: {0}"),
        K::DgAnuncios => ("Anúncios embutidos: {0}, válidos: {1}", "Embedded ads: {0}, valid: {1}"),
        K::Sim => ("sim", "yes"),
        K::Nao => ("não", "no"),
        K::Ok => ("ok", "ok"),
        K::Falha => ("FALHA ({0})", "FAILED ({0})"),
        K::OptEnv => ("variável de ambiente", "environment variable"),
        K::OptFlag => ("flag --sem-anuncios", "--sem-anuncios flag"),
        K::OptConfig => ("arquivo de config", "config file"),
        K::Nenhum => ("nenhum", "none"),
        _ => return None,
    })
}

/// O quê: o par (pt, en) de uma chave. Onde: `t`.
fn par(k: K) -> (&'static str, &'static str) {
    par_a(k).or_else(|| par_b(k)).or_else(|| par_c(k)).or_else(|| par_d(k)).unwrap_or(("?", "?"))
}

/// O quê: texto cru (com marcadores) no idioma. Onde: `tf` e mensagens sem argumento.
pub fn t(l: Idioma, k: K) -> &'static str {
    let (pt, en) = par(k);
    match l {
        Idioma::Pt => pt,
        Idioma::En => en,
    }
}

/// O quê: texto no idioma com `{0}`, `{1}`... trocados por `args`. Onde: toda mensagem com dados.
pub fn tf(l: Idioma, k: K, args: &[&str]) -> String {
    let mut s = t(l, k).to_string();
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), a);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAS: &[K] = &[
        K::ColNome,
        K::ColDominio,
        K::ColStatus,
        K::StAtivo,
        K::StPendente,
        K::StSemMcp,
        K::ListaRodape,
        K::IdVazio,
        K::IdDesconhecido,
        K::SemEndpointPendente,
        K::SemEndpointSemMcp,
        K::ExigeChave,
        K::ErroRede,
        K::ErroProtocolo,
        K::ErroGrande,
        K::ErroTimeout,
        K::ErroRecusado,
        K::ErroSemEndpoint,
        K::PreviaTitulo,
        K::NotaEstatica,
        K::NotaViva,
        K::Leitura,
        K::Escrita,
        K::AvisoEscrita,
        K::ChaveDefinida,
        K::ChaveFormato,
        K::EnClaude,
        K::EnCursor,
        K::EnVscode,
        K::EnNada,
        K::EnEnv,
        K::EnOauth,
        K::EnAplicarSo,
        K::EnSemClaude,
        K::EnAviso,
        K::EnOk,
        K::EnFalhou,
        K::DisClaude,
        K::DisManual,
        K::DisOk,
        K::AnContexto,
        K::AnBloqJson,
        K::AnBloqTty,
        K::AnBloqOptout,
        K::AnNenhum,
        K::AnEntregue,
        K::AnCandidato,
        K::DgCatalogo,
        K::DgOrdem,
        K::DgConector,
        K::DgOptout,
        K::DgTty,
        K::DgAnuncios,
        K::Sim,
        K::Nao,
        K::Ok,
        K::Falha,
        K::OptEnv,
        K::OptFlag,
        K::OptConfig,
        K::Nenhum,
    ];

    /// O quê: marcadores `{n}` presentes num texto, ordenados. Onde: teste de paridade.
    fn marcadores(s: &str) -> Vec<char> {
        let mut v: Vec<char> = s
            .match_indices('{')
            .filter_map(|(i, _)| s[i + 1..].chars().next().filter(char::is_ascii_digit))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    #[test]
    fn toda_chave_tem_texto_nos_dois_idiomas_com_os_mesmos_marcadores() {
        for k in TODAS {
            let (pt, en) = par(*k);
            assert!(pt != "?" && en != "?" && !pt.is_empty() && !en.is_empty(), "{k:?}");
            assert_eq!(marcadores(pt), marcadores(en), "{k:?}");
        }
    }

    #[test]
    fn tf_preenche_marcadores() {
        let s = tf(Idioma::En, K::IdDesconhecido, &["x", "a, b"]);
        assert!(s.contains("\"x\"") && s.contains("a, b") && !s.contains('{'));
    }
}
