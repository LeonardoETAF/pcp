//! Modelos de persistência das tabelas de entrada. Apenas dados — sem regra de negócio.
//! Datas de negócio são `NaiveDate` (CLAUDE.md §1); `ingerido_em` é o instante de carga.

use chrono::{DateTime, NaiveDate, Utc};

/// Linha de `pcp.vendas_dia` lida do banco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendaDia {
    pub id: i64,
    pub dt_ref: NaiveDate,
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub qtd_vendida: i32,
    pub is_personalizado: bool,
    pub ingerido_em: DateTime<Utc>,
}

/// Dados de entrada de uma venda (sem `id`/`ingerido_em`, gerados pelo banco).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NovaVendaDia {
    pub dt_ref: NaiveDate,
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub qtd_vendida: i32,
    pub is_personalizado: bool,
}

/// Linha de `pcp.estoque_snapshot` lida do banco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstoqueSnapshot {
    pub dt_ref: NaiveDate,
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub qtd_estoque: i32,
    pub qtd_reserva: i32,
    pub qtd_disponivel: i32,
    pub estoque_min_erp: Option<i32>,
    pub fora_de_linha: bool,
    pub ingerido_em: DateTime<Utc>,
}

/// Dados de entrada de um snapshot (sem `ingerido_em`, gerado pelo banco).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NovoEstoqueSnapshot {
    pub dt_ref: NaiveDate,
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub qtd_estoque: i32,
    pub qtd_reserva: i32,
    pub qtd_disponivel: i32,
    pub estoque_min_erp: Option<i32>,
    pub fora_de_linha: bool,
}
