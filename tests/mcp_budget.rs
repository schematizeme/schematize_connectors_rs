//! O BUDGET.md precisa declarar o teto de memória da prévia, igual ao calculado pelo código.

use connectors::mcp::PreviaViva;

#[test]
fn budget_declara_o_teto_de_memoria_da_previa() {
    let md = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/BUDGET.md"))
        .expect("BUDGET.md existe");
    let mib = PreviaViva::default().teto_memoria_bytes() / (1024 * 1024);
    assert_eq!(mib, 9, "default 1 MiB x 8 + 1 MiB");
    assert!(md.contains(&format!("teto de memória da prévia: {mib} MiB")), "BUDGET.md sem o teto");
}

#[test]
fn teto_cresce_com_max_bytes_e_nao_estoura() {
    let v = PreviaViva { max_bytes: usize::MAX, ..PreviaViva::default() };
    assert_eq!(v.teto_memoria_bytes(), usize::MAX);
    let w = PreviaViva { max_bytes: 2 * 1024 * 1024, ..PreviaViva::default() };
    assert_eq!(w.teto_memoria_bytes(), 17 * 1024 * 1024);
}
