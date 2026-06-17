//! Conector somente-leitura ao ERP One (`PostgreSQL` 9.5, schema `prd`) — fonte assíncrona atrás
//! do trait [`FonteDados`] (CLAUDE.md §1/§8; docs/integracao/acesso-direto-one.md). Fluxo:
//! consulta o One (read-only) → grava o cru no schema `bronze` → a ACL ([`crate::bronze`])
//! transforma para o domínio. SQL em **runtime**: o schema legado não entra no cache compile-time
//! do `SQLx`. Estoque = full refresh; vendas = **incremental** por `PEDV_DATC` com **janela
//! deslizante** (re-lê dias recentes p/ capturar cancelamentos). Sessão forçada read-only (§7).

use chrono::{Duration, NaiveDate};
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::{QueryBuilder, Row};

use pcp_db::{NovaVendaDia, NovoEstoqueSnapshot};

use crate::bronze::{acl_estoque, acl_venda, BronzeEstoque, BronzeVenda};
use crate::erro::ErroEtl;
use crate::fonte::FonteDados;

/// Estoque cru agregado por produto (F03005 + F03001), só produto acabado. `EST_QTDD` é o
/// disponível canônico; a reserva é derivada na ACL.
const SQL_ESTOQUE: &str = "\
SELECT p.itm_id AS itm_id, p.itm_sku AS itm_sku, p.itm_desc AS itm_desc, \
       ROUND(SUM(e.est_qtde))::int AS est_qtde, \
       ROUND(SUM(e.est_qtdd))::int AS est_qtdd, \
       ROUND(SUM(e.est_qtem))::int AS est_qtem, \
       BOOL_OR(COALESCE(e.est_flin, false)) AS est_flin, \
       BOOL_OR(COALESCE(p.itm_proda, false)) AS itm_proda \
FROM prd.f03005 e JOIN prd.f03001 p ON p.itm_id = e.est_itm \
WHERE p.itm_gpprd = 'PRODUTO_ACABADO' \
GROUP BY p.itm_id, p.itm_sku, p.itm_desc";

/// Vendas cru = itens de pedido NÃO cancelados, consolidados por (data do pedido, produto).
/// `$1` = data inicial da janela.
const SQL_VENDAS: &str = "\
SELECT p.pedv_datc::date AS pedv_datc, i.itmp_prd AS itmp_prd, \
       prod.itm_sku AS itm_sku, prod.itm_desc AS itm_desc, \
       ROUND(SUM(i.itmp_qnt))::int AS itmp_qnt, \
       BOOL_OR(COALESCE(prod.itm_proda, false)) AS itm_proda \
FROM prd.f05001 i \
JOIN prd.f05002 p ON p.pedv_id = i.itmp_pedv \
JOIN prd.f03001 prod ON prod.itm_id = i.itmp_prd \
WHERE i.itmp_stpd <> 'CANCELADO' AND i.itmp_dcan IS NULL AND p.pedv_dcan IS NULL \
  AND prod.itm_gpprd = 'PRODUTO_ACABADO' AND p.pedv_datc >= $1 \
GROUP BY p.pedv_datc::date, i.itmp_prd, prod.itm_sku, prod.itm_desc";

/// Marca-d'água da fonte de vendas em `bronze.sincronizacao`.
const FONTE_VENDAS: &str = "vendas";
/// Limite de linhas por lote no INSERT em batch (folga sobre o teto de parâmetros do Postgres).
const LOTE: usize = 5_000;

/// Parâmetros do ciclo de ingestão do One.
#[derive(Debug, Clone, Copy)]
pub struct OpcoesOne {
    /// Data de referência do snapshot (normalmente hoje).
    pub data_ref: NaiveDate,
    /// Profundidade do backfill na primeira sincronização (sem marca-d'água).
    pub backfill_dias: i64,
    /// Janela deslizante re-lida a cada ciclo, p/ capturar cancelamentos de pedidos recentes.
    pub janela_deslizante_dias: i64,
}

/// Fonte de dados por consulta direta ao One. Lê o One (`one`) e grava o cru no PCP (`pcp`,
/// schema `bronze`); a marca-d'água torna as vendas incrementais entre ciclos.
pub struct FonteConsultaOne {
    one: PgPool,
    pcp: PgPool,
    opcoes: OpcoesOne,
}

impl FonteConsultaOne {
    /// Conecta ao One (URL read-only do ambiente — §7.4) reusando o pool do PCP para o bronze.
    /// Cada conexão ao One entra em transação somente-leitura e com `statement_timeout`.
    ///
    /// # Errors
    /// [`ErroEtl::One`] se a conexão inicial falhar.
    pub async fn conectar(one_url: &str, pcp: PgPool, opcoes: OpcoesOne) -> Result<Self, ErroEtl> {
        let one = PgPoolOptions::new()
            .max_connections(2)
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
            .connect(one_url)
            .await?;
        Ok(Self { one, pcp, opcoes })
    }

    /// Início da janela de vendas: backfill na 1ª vez; senão, marca-d'água − janela deslizante.
    async fn inicio_janela_vendas(&self) -> Result<NaiveDate, ErroEtl> {
        let marca: Option<NaiveDate> =
            sqlx::query("SELECT marca_dagua FROM bronze.sincronizacao WHERE fonte = $1")
                .bind(FONTE_VENDAS)
                .fetch_optional(&self.pcp)
                .await?
                .and_then(|r| {
                    r.try_get::<Option<NaiveDate>, _>("marca_dagua")
                        .ok()
                        .flatten()
                });
        let inicio = match marca {
            Some(m) => m - Duration::days(self.opcoes.janela_deslizante_dias),
            None => self.opcoes.data_ref - Duration::days(self.opcoes.backfill_dias),
        };
        Ok(inicio)
    }

    /// Lê o estoque cru do One.
    async fn estoque_cru(&self) -> Result<Vec<BronzeEstoque>, ErroEtl> {
        let linhas = sqlx::query(SQL_ESTOQUE).fetch_all(&self.one).await?;
        Ok(linhas
            .iter()
            .map(|r| BronzeEstoque {
                itm_id: r.get::<i64, _>("itm_id"),
                itm_sku: texto(r, "itm_sku"),
                itm_desc: texto(r, "itm_desc"),
                est_qtde: inteiro(r, "est_qtde"),
                est_qtdd: inteiro(r, "est_qtdd"),
                est_qtem: r.try_get::<Option<i32>, _>("est_qtem").ok().flatten(),
                est_flin: booleano(r, "est_flin"),
                itm_proda: booleano(r, "itm_proda"),
            })
            .collect())
    }

    /// Lê as vendas cruas do One a partir de `desde`.
    async fn vendas_cru(&self, desde: NaiveDate) -> Result<Vec<BronzeVenda>, ErroEtl> {
        let linhas = sqlx::query(SQL_VENDAS)
            .bind(desde)
            .fetch_all(&self.one)
            .await?;
        Ok(linhas
            .iter()
            .map(|r| BronzeVenda {
                pedv_datc: r.get::<NaiveDate, _>("pedv_datc"),
                itmp_prd: r.get::<i64, _>("itmp_prd"),
                itm_sku: texto(r, "itm_sku"),
                itm_desc: texto(r, "itm_desc"),
                itmp_qnt: inteiro(r, "itmp_qnt"),
                itm_proda: booleano(r, "itm_proda"),
            })
            .collect())
    }

    /// Grava o estoque cru no bronze (full refresh do dia: troca a `data_ref`).
    async fn landar_estoque(&self, cru: &[BronzeEstoque]) -> Result<(), ErroEtl> {
        let data_ref = self.opcoes.data_ref;
        let mut tx = self.pcp.begin().await?;
        sqlx::query("DELETE FROM bronze.one_estoque WHERE data_ref = $1")
            .bind(data_ref)
            .execute(&mut *tx)
            .await?;
        for lote in cru.chunks(LOTE) {
            let mut qb = QueryBuilder::new(
                "INSERT INTO bronze.one_estoque \
                 (data_ref, itm_id, itm_sku, itm_desc, est_qtde, est_qtdd, est_qtem, est_flin, itm_proda) ",
            );
            qb.push_values(lote, |mut b, r| {
                b.push_bind(data_ref)
                    .push_bind(r.itm_id)
                    .push_bind(r.itm_sku.as_deref())
                    .push_bind(r.itm_desc.as_deref())
                    .push_bind(r.est_qtde)
                    .push_bind(r.est_qtdd)
                    .push_bind(r.est_qtem)
                    .push_bind(r.est_flin)
                    .push_bind(r.itm_proda);
            });
            qb.build().execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Grava as vendas cruas no bronze (full refresh da janela: troca tudo a partir de `desde`).
    async fn landar_vendas(&self, desde: NaiveDate, cru: &[BronzeVenda]) -> Result<(), ErroEtl> {
        let mut tx = self.pcp.begin().await?;
        sqlx::query("DELETE FROM bronze.one_venda WHERE pedv_datc >= $1")
            .bind(desde)
            .execute(&mut *tx)
            .await?;
        for lote in cru.chunks(LOTE) {
            let mut qb = QueryBuilder::new(
                "INSERT INTO bronze.one_venda \
                 (pedv_datc, itmp_prd, itm_sku, itm_desc, itmp_qnt, itm_proda) ",
            );
            qb.push_values(lote, |mut b, r| {
                b.push_bind(r.pedv_datc)
                    .push_bind(r.itmp_prd)
                    .push_bind(r.itm_sku.as_deref())
                    .push_bind(r.itm_desc.as_deref())
                    .push_bind(r.itmp_qnt)
                    .push_bind(r.itm_proda);
            });
            qb.build().execute(&mut *tx).await?;
        }
        sqlx::query(
            "INSERT INTO bronze.sincronizacao (fonte, marca_dagua, atualizado_em) \
             VALUES ($1, $2, now()) \
             ON CONFLICT (fonte) DO UPDATE SET marca_dagua = EXCLUDED.marca_dagua, atualizado_em = now()",
        )
        .bind(FONTE_VENDAS)
        .bind(self.opcoes.data_ref)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Sincroniza as fontes COMPLEMENTARES (faturada + produção) no bronze a partir de `desde`
    /// (mapeamento §10). Não fazem parte da demanda — visibilidade e uso futuro do motor.
    /// Retorna `(linhas_fatura, linhas_producao)`.
    ///
    /// # Errors
    /// [`ErroEtl`] em falha de consulta ao One ou gravação no bronze.
    pub async fn sincronizar_complementares(
        &self,
        desde: NaiveDate,
    ) -> Result<(u64, u64), ErroEtl> {
        let faturas = crate::complementar::sincronizar_faturas(&self.one, &self.pcp, desde).await?;
        let producao = crate::complementar::sincronizar_producao(&self.one, &self.pcp).await?;
        Ok((faturas, producao))
    }
}

impl FonteDados for FonteConsultaOne {
    async fn ler_vendas(&self) -> Result<Vec<NovaVendaDia>, ErroEtl> {
        let desde = self.inicio_janela_vendas().await?;
        let cru = self.vendas_cru(desde).await?;
        self.landar_vendas(desde, &cru).await?;
        Ok(cru.iter().map(acl_venda).collect())
    }

    async fn ler_snapshots(&self) -> Result<Vec<NovoEstoqueSnapshot>, ErroEtl> {
        let cru = self.estoque_cru().await?;
        self.landar_estoque(&cru).await?;
        let data_ref = self.opcoes.data_ref;
        Ok(cru.iter().map(|b| acl_estoque(b, data_ref)).collect())
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

/// Lê um booleano agregado (`BOOL_OR` pode vir nulo), assumindo `false` na ausência.
fn booleano(r: &PgRow, col: &str) -> bool {
    r.try_get::<Option<bool>, _>(col)
        .ok()
        .flatten()
        .unwrap_or(false)
}
