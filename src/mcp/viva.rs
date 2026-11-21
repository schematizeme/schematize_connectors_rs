//! Prévia ao vivo (stub até a implementação do fluxo).

use super::tipos::{ErroMcp, Previa, PreviaViva, Previador};
use crate::conector::Conector;
use crate::segredo::Segredo;

impl Previador for PreviaViva {
    /// O quê: stub provisório. Onde: CLI; substituído na próxima etapa.
    fn previa(&self, _c: &Conector, _s: Option<&Segredo>) -> Result<Previa, ErroMcp> {
        Err(ErroMcp::Protocolo("prévia viva ainda não implementada".into()))
    }
}
