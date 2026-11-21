# schematize connectors

Catálogo de **conectores MCP com prévia** e **anúncios da casa** para o schematize.
Esta é a fundação (U1): tipos, catálogo embutido e regras de anúncio — lógica **pura, sem rede**.
O cliente MCP (`src/mcp/`) e a CLI (`src/cli/`) vêm nas próximas unidades.

## Catálogo

`catalogo()` devolve uma fatia `&'static`, em ordem determinística: o conector **principal**
(`nebularws`) é sempre o primeiro e há exatamente um. Entradas: `nebularws`, `asender`
(MCP real, 32 ferramentas de prévia), `owiew`, `axuz` (endpoint pendente) e `canverly`
(sem MCP por enquanto). Nenhuma URL é inventada: endpoint pendente é `None`.

## Segurança por construção

- `Conector::validar()`: endpoint só `https://`, host igual ao domínio do conector ou
  subdomínio dele, sem userinfo, porta só 443, sem query/fragmento, ASCII estrito (anti-SSRF
  e anti-lookalike).
- Segredo **nunca** é gravado nem logado: só se referencia por **nome** de env var
  (`ASENDER_API_KEY`). `resolver_env` devolve um `Segredo` cujo `Debug`/`Display` não
  imprimem o valor; `mascarar` mostra no máximo `sk_l…wxyz`.

## Anúncios

`escolher(&Contexto)` é pura e determinística (sem `rand`, sem relógio). Regras: nunca em
modo JSON nem sem TTY; nunca com opt-out (`SCHEMATIZE_SEM_ANUNCIOS=1|true|yes|sim`); nunca de
conector já ativo; respeita expiração; no máximo 1 por sessão; o principal tem prioridade
(sessões pares); rótulo obrigatório `Anúncio · da casa` / `Ad · from us`.

> **Regra inegociável: anúncio NUNCA vai para prompt/contexto de agente.** Gasta token e é
> vetor de prompt injection. A única saída é `render_texto` (texto puro, para terminal).

Os anúncios atuais são **modelos** aguardando a copy do dono.

## Desenvolvimento

```sh
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

O CI remoto (`.github/workflows/ci.yml`) roda só sob `workflow_dispatch` ou em tag (`hml-*`, `v*`).
