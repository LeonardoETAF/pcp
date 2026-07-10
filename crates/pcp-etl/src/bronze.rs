//! Camada anticorrupção (ACL): transforma o cru do One (`bronze`, nomes `Fxxxxx`/colunas
//! crípticas) no contrato de domínio do PCP (nomes honestos, tipos certos — doc 05 §2). As
//! funções de transformação são **puras** (struct → struct), testáveis sem banco — é aqui que
//! a divergência entre o ERP legado e o domínio é resolvida e documentada (CLAUDE.md §1/§8).

use chrono::NaiveDate;

use pcp_db::{NovaVendaDia, NovoEstoqueSnapshot};

/// Linha crua de estoque do One: **uma por linha de estoque** (`EST_ID` = item × configuração).
/// Espelha `bronze.one_estoque`.
#[derive(Debug, Clone)]
pub struct BronzeEstoque {
    pub est_id: i64,
    pub est_itm: i64,
    pub est_cnf: Option<i64>,
    pub est_dconf: Option<String>,
    pub itm_sku: Option<String>,
    pub itm_desc: Option<String>,
    pub est_qtde: i32,
    pub est_qtdd: i32,
    pub est_qtem: Option<i32>,
    pub est_flin: bool,
    pub itm_proda: bool,
}

/// Linha crua de venda do One (kardex, líquido dia×linha de estoque). Espelha `bronze.one_venda`.
#[derive(Debug, Clone)]
pub struct BronzeVenda {
    pub cdx_datc: NaiveDate,
    pub cdx_estq: i64,
    pub itm_sku: Option<String>,
    pub itm_desc: Option<String>,
    pub est_dconf: Option<String>,
    pub cdx_qtd: i32,
    pub itm_proda: bool,
}

/// ACL do estoque: `bronze.one_estoque` → `NovoEstoqueSnapshot` (doc 05 §2.2).
///
/// `codigo_estoque ← EST_ID`, a linha de estoque (item × configuração) — é o `codigo_estoque` do
/// legado, e não o `ITM_ID`: os produtos de referência do PRD §11 (6797, 10001, 10473) só existem
/// como `EST_ID`. `configuracao ← EST_DCONF` (§12), ex.: `"COR DO PRODUTO: PRETO"`.
///
/// Decisão documentada: `qtd_disponivel ← EST_QTDD` (canônico, confirmado pelo suporte) e
/// `qtd_estoque ← EST_QTDE`; a **reserva é derivada** (`estoque − disponível`) para honrar a
/// invariante do contrato (`disponivel = estoque − reserva`), já que as três quantidades do One
/// são `double` independentes e não fecham após arredondar.
#[must_use]
pub fn acl_estoque(b: &BronzeEstoque, data_ref: NaiveDate) -> NovoEstoqueSnapshot {
    NovoEstoqueSnapshot {
        dt_ref: data_ref,
        codigo_estoque: b.est_id.to_string(),
        sku: limpar(b.itm_sku.as_deref()),
        produto: limpar(b.itm_desc.as_deref()),
        configuracao: limpar(b.est_dconf.as_deref()),
        qtd_estoque: b.est_qtde,
        qtd_reserva: b.est_qtde - b.est_qtdd,
        qtd_disponivel: b.est_qtdd,
        estoque_min_erp: b.est_qtem,
        fora_de_linha: b.est_flin,
    }
}

/// ACL da venda: `bronze.one_venda` → `NovaVendaDia` (doc 05 §2.1). Mesma chave do snapshot
/// (`EST_ID`), então venda e estoque falam do mesmo produto-cor. `is_personalizado` é atributo do
/// item (`ITM_PRODA`).
#[must_use]
pub fn acl_venda(b: &BronzeVenda) -> NovaVendaDia {
    NovaVendaDia {
        dt_ref: b.cdx_datc,
        codigo_estoque: b.cdx_estq.to_string(),
        sku: limpar(b.itm_sku.as_deref()),
        produto: limpar(b.itm_desc.as_deref()),
        configuracao: limpar(b.est_dconf.as_deref()),
        qtd_vendida: b.cdx_qtd,
        is_personalizado: b.itm_proda,
    }
}

/// Normaliza texto opcional do One: branco → `None`.
fn limpar(t: Option<&str>) -> Option<String> {
    t.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn data() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 6, 17).unwrap()
    }

    #[test]
    fn estoque_deriva_reserva_para_honrar_invariante() {
        // EST_QTDD (disponível) é canônico; reserva = estoque − disponível, sempre coerente.
        let b = BronzeEstoque {
            est_id: 923,
            est_itm: 40,
            est_cnf: Some(7),
            est_dconf: Some("COR DO PRODUTO: PRETO".to_owned()),
            itm_sku: Some(" CANUDO ".to_owned()),
            itm_desc: Some("CANUDO P LISO".to_owned()),
            est_qtde: 1000,
            est_qtdd: 800,
            est_qtem: Some(50),
            est_flin: false,
            itm_proda: true,
        };
        let s = acl_estoque(&b, data());
        assert_eq!(s.codigo_estoque, "923");
        assert_eq!(s.sku.as_deref(), Some("CANUDO")); // trim aplicado
        assert_eq!(s.qtd_estoque, 1000);
        assert_eq!(s.qtd_disponivel, 800);
        assert_eq!(s.qtd_reserva, 200);
        // invariante do contrato (doc 05 §2.2)
        assert_eq!(s.qtd_disponivel, s.qtd_estoque - s.qtd_reserva);
        assert_eq!(s.configuracao.as_deref(), Some("COR DO PRODUTO: PRETO"));
        assert_eq!(s.estoque_min_erp, Some(50));
    }

    #[test]
    fn estoque_disponivel_maior_que_fisico_gera_reserva_negativa_coerente() {
        // O One pode ter disponível > físico; a invariante ainda fecha (reserva negativa).
        let b = BronzeEstoque {
            est_id: 1,
            est_itm: 1,
            est_cnf: None,
            est_dconf: None,
            itm_sku: None,
            itm_desc: Some("   ".to_owned()), // só espaços → None
            est_qtde: 0,
            est_qtdd: 40,
            est_qtem: None,
            est_flin: true,
            itm_proda: false,
        };
        let s = acl_estoque(&b, data());
        assert_eq!(s.qtd_reserva, -40);
        assert_eq!(s.qtd_disponivel, s.qtd_estoque - s.qtd_reserva);
        assert!(s.produto.is_none());
        assert!(s.fora_de_linha);
    }

    #[test]
    fn venda_usa_a_linha_de_estoque_como_chave_e_carrega_a_configuracao() {
        let b = BronzeVenda {
            cdx_datc: data(),
            cdx_estq: 10001,
            itm_sku: Some("TLPL".to_owned()),
            itm_desc: Some("TULIPA LISO".to_owned()),
            est_dconf: Some("COR DO PRODUTO: CRISTAL".to_owned()),
            cdx_qtd: 4800,
            itm_proda: true,
        };
        let v = acl_venda(&b);
        // Mesma chave do snapshot: venda e estoque falam do mesmo produto-cor.
        assert_eq!(v.codigo_estoque, "10001");
        assert_eq!(v.dt_ref, data());
        assert_eq!(v.qtd_vendida, 4800);
        assert!(v.is_personalizado);
        assert_eq!(v.configuracao.as_deref(), Some("COR DO PRODUTO: CRISTAL"));
    }
}
