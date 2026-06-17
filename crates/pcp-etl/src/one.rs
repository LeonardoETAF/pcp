//! Conector somente-leitura ao ERP One (`PostgreSQL` 9.5, schema `prd`) — camada anticorrupção
//! (CLAUDE.md §1/§8; docs/integracao/acesso-direto-one.md). Lê o cru do One e o transforma no
//! contrato de entrada do PCP (doc 05 §2). O SQL é executado em **runtime**: o schema legado do
//! One não entra no cache compile-time do `SQLx` — é fonte externa, não pertence ao domínio. A
//! sessão é forçada **somente-leitura** (§7), defesa extra além do usuário `GRANT SELECT`.

use chrono::NaiveDate;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::Row;

use pcp_db::{NovaVendaDia, NovoEstoqueSnapshot};

use crate::erro::ErroEtl;

/// Snapshot completo do estoque (F03005 + F03001), agregado por produto: soma das configurações
/// e `fora_de_linha` por `BOOL_OR`. Filtra produto acabado (mapeamento §1). `EST_QTDD` é o
/// disponível canônico (confirmado pelo suporte); `EST_QTDE` é o físico. A **reserva** é
/// derivada como resíduo (`estoque − disponível`) na transformação — ver `ler_snapshot`.
const SQL_SNAPSHOT: &str = "\
SELECT p.itm_id AS itm_id, p.itm_sku AS sku, p.itm_desc AS produto, \
       ROUND(SUM(e.est_qtde))::int AS qtd_estoque, \
       ROUND(SUM(e.est_qtdd))::int AS qtd_disponivel, \
       ROUND(SUM(e.est_qtem))::int AS estoque_min_erp, \
       BOOL_OR(COALESCE(e.est_flin, false)) AS fora_de_linha \
FROM prd.f03005 e JOIN prd.f03001 p ON p.itm_id = e.est_itm \
WHERE p.itm_gpprd = 'PRODUTO_ACABADO' \
GROUP BY p.itm_id, p.itm_sku, p.itm_desc";

/// Vendas = itens de pedido NÃO cancelados (mapeamento §2, opção B), consolidados por (data do
/// pedido, produto). `dt_ref` ← `PEDV_DATC`; exclui item/pedido cancelado. Restringe a produto
/// acabado para manter o mesmo universo do snapshot. `$1` = data inicial da janela.
const SQL_VENDAS: &str = "\
SELECT p.pedv_datc::date AS dt_ref, i.itmp_prd AS codigo, \
       prod.itm_sku AS sku, prod.itm_desc AS produto, \
       ROUND(SUM(i.itmp_qnt))::int AS qtd_vendida, \
       BOOL_OR(COALESCE(prod.itm_proda, false)) AS is_personalizado \
FROM prd.f05001 i \
JOIN prd.f05002 p ON p.pedv_id = i.itmp_pedv \
JOIN prd.f03001 prod ON prod.itm_id = i.itmp_prd \
WHERE i.itmp_stpd <> 'CANCELADO' AND i.itmp_dcan IS NULL AND p.pedv_dcan IS NULL \
  AND prod.itm_gpprd = 'PRODUTO_ACABADO' AND p.pedv_datc >= $1 \
GROUP BY p.pedv_datc::date, i.itmp_prd, prod.itm_sku, prod.itm_desc";

/// Fonte de dados por consulta direta ao One. Pool dedicado, sessão somente-leitura.
pub struct FonteConsultaOne {
    pool: PgPool,
}

impl FonteConsultaOne {
    /// Conecta ao One com URL somente-leitura vinda do ambiente (§7.4). Cada conexão entra em
    /// modo transação somente-leitura e com `statement_timeout`, para nunca escrever no legado.
    ///
    /// # Errors
    /// [`ErroEtl::One`] se a conexão inicial falhar.
    pub async fn conectar(url: &str, max_conexoes: u32) -> Result<Self, ErroEtl> {
        let pool = PgPoolOptions::new()
            .max_connections(max_conexoes)
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    sqlx::query("SET default_transaction_read_only = on")
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("SET statement_timeout = '180s'")
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(url)
            .await?;
        Ok(Self { pool })
    }

    /// Lê o snapshot de estoque do dia `data_ref` (full refresh — o One só guarda o saldo atual).
    ///
    /// # Errors
    /// [`ErroEtl::One`] se a consulta falhar.
    pub async fn ler_snapshot(
        &self,
        data_ref: NaiveDate,
    ) -> Result<Vec<NovoEstoqueSnapshot>, ErroEtl> {
        let linhas = sqlx::query(SQL_SNAPSHOT).fetch_all(&self.pool).await?;
        Ok(linhas
            .iter()
            .map(|r| {
                let qtd_estoque = inteiro(r, "qtd_estoque");
                let qtd_disponivel = inteiro(r, "qtd_disponivel");
                NovoEstoqueSnapshot {
                    dt_ref: data_ref,
                    codigo_estoque: r.get::<i64, _>("itm_id").to_string(),
                    sku: texto(r, "sku"),
                    produto: texto(r, "produto"),
                    configuracao: None, // agregado por produto, não por configuração
                    qtd_estoque,
                    // Reserva derivada p/ honrar a invariante do contrato (doc 05 §2.2):
                    // disponivel = estoque − reserva. O One traz EST_QTDR independente, mas as
                    // três quantidades são doubles e não fecham após arredondar; disponível
                    // (EST_QTDD) é o canônico (suporte) — a reserva absorve o resíduo.
                    qtd_reserva: qtd_estoque - qtd_disponivel,
                    qtd_disponivel,
                    estoque_min_erp: r
                        .try_get::<Option<i32>, _>("estoque_min_erp")
                        .ok()
                        .flatten(),
                    fora_de_linha: r
                        .try_get::<Option<bool>, _>("fora_de_linha")
                        .ok()
                        .flatten()
                        .unwrap_or(false),
                }
            })
            .collect())
    }

    /// Lê as vendas (pedidos não cancelados) a partir de `desde` (inclusive).
    ///
    /// # Errors
    /// [`ErroEtl::One`] se a consulta falhar.
    pub async fn ler_vendas(&self, desde: NaiveDate) -> Result<Vec<NovaVendaDia>, ErroEtl> {
        let linhas = sqlx::query(SQL_VENDAS)
            .bind(desde)
            .fetch_all(&self.pool)
            .await?;
        Ok(linhas
            .iter()
            .map(|r| NovaVendaDia {
                dt_ref: r.get::<NaiveDate, _>("dt_ref"),
                codigo_estoque: r.get::<i64, _>("codigo").to_string(),
                sku: texto(r, "sku"),
                produto: texto(r, "produto"),
                configuracao: None, // consolidado por produto (o motor soma as variações)
                qtd_vendida: inteiro(r, "qtd_vendida"),
                is_personalizado: r
                    .try_get::<Option<bool>, _>("is_personalizado")
                    .ok()
                    .flatten()
                    .unwrap_or(false),
            })
            .collect())
    }
}

/// Lê uma coluna textual opcional, normalizando branco → `None`.
fn texto(r: &PgRow, col: &str) -> Option<String> {
    r.try_get::<Option<String>, _>(col)
        .ok()
        .flatten()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// Lê uma quantidade inteira agregada (SUM pode vir nula quando todas as parcelas são nulas).
fn inteiro(r: &PgRow, col: &str) -> i32 {
    r.try_get::<Option<i32>, _>(col).ok().flatten().unwrap_or(0)
}
