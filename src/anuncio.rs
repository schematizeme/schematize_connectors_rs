//! Anúncios da casa: lógica PURA e determinística, sem rede, sem relógio, sem `rand`.
//!
//! # Regra inegociável: anúncio NUNCA vai para prompt/contexto de agente
//!
//! Anúncio é texto para o TERMINAL/tela de um humano. Mandá-lo a um modelo (1) gasta token à
//! toa e (2) é vetor de prompt injection: texto de terceiros entrando no contexto do agente.
//! Por isso a única saída pública é [`render_texto`] (texto puro para terminal, sem markup) e
//! [`escolher`] recusa `modo_json` e ausência de TTY — os dois modos em que um agente lê.

use crate::catalogo::buscar;
use crate::conector::validar_https_do_dominio;
use crate::erro::ErroConector;
use serde::Serialize;

/// Onde o anúncio aparece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Slot {
    /// Tela inicial do app.
    TelaInicial,
    /// Estado vazio (lista sem itens).
    EstadoVazio,
    /// Banner discreto na CLI.
    BannerCli,
}

/// Idioma de exibição.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idioma {
    /// Português.
    Pt,
    /// Inglês.
    En,
}

/// Data civil (`AAAA-MM-DD`), ordenável, sem fuso: o relógio é injetado pelo chamador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Data {
    /// Ano.
    pub ano: u16,
    /// Mês (1-12).
    pub mes: u8,
    /// Dia (1-31).
    pub dia: u8,
}

impl Data {
    /// O quê: interpreta `AAAA-MM-DD` estrito (4-2-2 dígitos ASCII, mês 1-12, dia 1-31).
    /// Onde: expiração de anúncio e parse do "hoje" injetado pela CLI.
    pub fn parse(s: &str) -> Option<Data> {
        let b = s.as_bytes();
        let ok = b.len() == 10 && b[4] == b'-' && b[7] == b'-';
        if !ok || !s.bytes().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
            return None;
        }
        let ano = s[0..4].parse().ok()?;
        let mes: u8 = s[5..7].parse().ok()?;
        let dia: u8 = s[8..10].parse().ok()?;
        ((1..=12).contains(&mes) && (1..=31).contains(&dia)).then_some(Data { ano, mes, dia })
    }
}

/// Um anúncio embutido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Anuncio {
    /// Identificador estável.
    pub id: &'static str,
    /// Conector anunciado (id do catálogo).
    pub conector_id: &'static str,
    /// Texto em português.
    pub texto_pt: &'static str,
    /// Texto em inglês.
    pub texto_en: &'static str,
    /// Link (https do domínio do conector).
    pub link: &'static str,
    /// Imagem opcional (reservada à UI gráfica; o terminal ignora).
    pub imagem: Option<&'static str>,
    /// Slots onde pode aparecer.
    pub slots: &'static [Slot],
    /// Última data de exibição (`AAAA-MM-DD`, inclusive); `None` = sem expiração.
    pub expira: Option<&'static str>,
}

/// Slots padrão dos anúncios-modelo.
const TODOS: &[Slot] = &[Slot::TelaInicial, Slot::EstadoVazio, Slot::BannerCli];

/// O quê: monta o anúncio-modelo de um conector. Onde: tabela `ANUNCIOS`.
const fn modelo(
    id: &'static str,
    conector_id: &'static str,
    texto_pt: &'static str,
    texto_en: &'static str,
    link: &'static str,
) -> Anuncio {
    Anuncio { id, conector_id, texto_pt, texto_en, link, imagem: None, slots: TODOS, expira: None }
}

/// Um anúncio-modelo por conector, na ordem do catálogo.
static ANUNCIOS: [Anuncio; 5] = [
    modelo(
        "nebularws-modelo",
        "nebularws",
        "NebulaRWS, projeto da casa. [modelo — aguardando copy do dono]",
        "NebulaRWS, a project from us. [modelo — aguardando copy do dono]",
        "https://nebularws.com",
    ),
    modelo(
        "asender-modelo",
        "asender",
        "Asender: comunicação, marketing e atendimento para empresas.",
        "Asender: communication, marketing and customer service for businesses.",
        "https://asender.net",
    ),
    modelo(
        "owiew-modelo",
        "owiew",
        "Owiew: do clique ao faturamento.",
        "Owiew: from click to revenue.",
        "https://owiew.com",
    ),
    modelo(
        "axuz-modelo",
        "axuz",
        "Axuz, projeto da casa. [modelo — aguardando copy do dono]",
        "Axuz, a project from us. [modelo — aguardando copy do dono]",
        "https://axuz.net",
    ),
    modelo(
        "canverly-modelo",
        "canverly",
        "Canverly: rede de portais editoriais.",
        "Canverly: a network of editorial portals.",
        "https://canverly.net",
    ),
];

/// Anúncios embutidos. O quê: fatia estática. Onde: [`escolher`] e a CLI.
pub fn anuncios() -> &'static [Anuncio] {
    &ANUNCIOS
}

/// Contexto de uma decisão de exibição (tudo injetado: a função é pura).
#[derive(Debug, Clone, Copy)]
pub struct Contexto<'a> {
    /// Onde se quer exibir.
    pub slot: Slot,
    /// Idioma do usuário.
    pub idioma: Idioma,
    /// `true` se a saída é um terminal interativo.
    pub tty: bool,
    /// `true` se a saída é JSON (consumo por máquina/agente).
    pub modo_json: bool,
    /// `true` se o usuário desligou anúncios.
    pub optout: bool,
    /// Ids dos conectores que o usuário já tem ativos.
    pub conectores_ativos: &'a [&'a str],
    /// Contador de sessões (semente da rotação).
    pub sessao_n: u32,
    /// Quantos anúncios já saíram NESTA sessão (teto: 1).
    pub exibidos_na_sessao: u32,
    /// "Hoje", injetado.
    pub agora: Data,
}

/// Rótulo obrigatório que acompanha todo anúncio.
///
/// O quê: `Anúncio · da casa` / `Ad · from us`. Onde: [`render_texto`] e UIs.
pub fn rotulo(idioma: Idioma) -> &'static str {
    match idioma {
        Idioma::Pt => "Anúncio · da casa",
        Idioma::En => "Ad · from us",
    }
}

impl Anuncio {
    /// O quê: confere que o conector existe e o link é https do domínio dele.
    /// Onde: filtro de [`escolher`] e teste de invariante.
    pub fn validar(&self) -> Result<(), ErroConector> {
        let c = buscar(self.conector_id).ok_or(ErroConector::Incoerente("conector inexistente"))?;
        validar_https_do_dominio(self.link, c.dominio)
    }

    /// O quê: `true` se já expirou em `hoje`; data ilegível conta como expirada (fail-closed).
    /// Onde: filtro de [`escolher`].
    fn expirado(&self, hoje: Data) -> bool {
        match self.expira {
            None => false,
            Some(s) => Data::parse(s).is_none_or(|d| hoje > d),
        }
    }

    /// O quê: elegibilidade independente de rotação (slot, expiração, conector, link).
    /// Onde: `escolher`.
    fn elegivel(&self, ctx: &Contexto<'_>) -> bool {
        self.slots.contains(&ctx.slot)
            && !self.expirado(ctx.agora)
            && !ctx.conectores_ativos.contains(&self.conector_id)
            && self.validar().is_ok()
    }
}

/// O quê: o conector do anúncio é o principal? Onde: prioridade em `escolher`.
fn e_principal(a: &Anuncio) -> bool {
    buscar(a.conector_id).is_some_and(|c| c.principal)
}

/// Decide qual anúncio mostrar (ou nenhum). PURA e determinística.
///
/// O quê: aplica as regras — nunca em `modo_json`/sem `tty`/com `optout`; no máx. 1 por
/// sessão; ignora conector ativo, expirado, fora do slot ou com link inválido. Rotação por
/// `sessao_n`: o principal ocupa as sessões pares; as ímpares rodam entre os demais (sem
/// principal elegível, roda entre todos).
/// Onde: telas do app e banner da CLI, uma vez por sessão.
pub fn escolher(ctx: &Contexto<'_>) -> Option<&'static Anuncio> {
    if ctx.modo_json || !ctx.tty || ctx.optout || ctx.exibidos_na_sessao >= 1 {
        return None;
    }
    let (principal, outros): (Vec<&Anuncio>, Vec<&Anuncio>) =
        anuncios().iter().filter(|a| a.elegivel(ctx)).partition(|a| e_principal(a));
    let n = ctx.sessao_n as usize;
    match (principal.first(), outros.len()) {
        (Some(p), 0) => Some(p),
        (Some(p), _) if n.is_multiple_of(2) => Some(p),
        (Some(_), k) => outros.get((n / 2) % k).copied(),
        (None, 0) => None,
        (None, k) => outros.get(n % k).copied(),
    }
}

/// Renderiza para terminal: texto puro, sem markup, com rótulo obrigatório.
///
/// O quê: `[<rótulo>] <texto> <link>`; caracteres de controle (ESC, etc.) são descartados.
/// Onde: ÚNICA saída pública de anúncio; destinada a humano em TTY, nunca a agente.
pub fn render_texto(a: &Anuncio, idioma: Idioma) -> String {
    let texto = match idioma {
        Idioma::Pt => a.texto_pt,
        Idioma::En => a.texto_en,
    };
    let bruto = format!("[{}] {} {}", rotulo(idioma), texto, a.link);
    bruto.chars().filter(|c| !c.is_control()).collect()
}

/// Lê o opt-out do ambiente por um leitor injetável.
///
/// O quê: `SCHEMATIZE_SEM_ANUNCIOS` em `1`/`true`/`yes`/`sim` (sem caixa, sem espaços) liga o opt-out.
/// Onde: montagem do [`Contexto`] na CLI/app (`|k| std::env::var(k).ok()`).
pub fn optout_de_env(get: impl Fn(&str) -> Option<String>) -> bool {
    get("SCHEMATIZE_SEM_ANUNCIOS")
        .is_some_and(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes" | "sim"))
}
