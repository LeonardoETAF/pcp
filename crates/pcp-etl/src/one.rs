//! Conector somente-leitura ao ERP One (`PostgreSQL` 9.5, schema `prd`) — fonte assíncrona atrás
//! do trait [`FonteDados`] (CLAUDE.md §1/§8; docs/integracao/acesso-direto-one.md). Fluxo:
//! consulta o One (read-only) → grava o cru no schema `bronze` → a ACL ([`crate::bronze`])
//! transforma para o domínio. SQL em **runtime**: o schema legado não entra no cache compile-time
//! do `SQLx`. Estoque = full refresh; vendas = **incremental** por `CDX_DATC` com **janela
//! deslizante** (re-lê dias recentes p/ capturar estornos). Sessão forçada read-only (§7).
//!
//! **Grão = linha de estoque** (`F03005.EST_ID` = item × configuração de cor), não o item: é o
//! `codigo_estoque` do legado (os produtos de referência do PRD §11 são `EST_ID`). Ver migration
//! `0015_grao_linha_de_estoque.sql`.

use chrono::{Duration, NaiveDate};
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::{QueryBuilder, Row};

use pcp_db::{NovaVendaDia, NovoEstoqueSnapshot};

use crate::bronze::{acl_estoque, acl_venda, BronzeEstoque, BronzeVenda};
use crate::erro::ErroEtl;
use crate::fonte::FonteDados;

/// Estoque cru, **uma linha por linha de estoque** (F03005 × F03001), só produto acabado. Sem
/// `GROUP BY`: agregar por item somaria as cores e apagaria a configuração. `EST_QTDD` é o
/// disponível canônico; a reserva é derivada na ACL.
const SQL_ESTOQUE: &str = "\
SELECT e.est_id AS est_id, e.est_itm AS est_itm, e.est_cnf AS est_cnf, e.est_dconf AS est_dconf, \
       p.itm_sku AS itm_sku, p.itm_desc AS itm_desc, \
       ROUND(e.est_qtde)::int AS est_qtde, \
       ROUND(e.est_qtdd)::int AS est_qtdd, \
       ROUND(e.est_qtem)::int AS est_qtem, \
       COALESCE(e.est_flin, false) AS est_flin, \
       COALESCE(p.itm_proda, false) AS itm_proda \
FROM prd.f03005 e JOIN prd.f03001 p ON p.itm_id = e.est_itm \
WHERE p.itm_gpprd = 'PRODUTO_ACABADO'";

/// Vendas cruas do **kardex** (F03007): é a única fonte que amarra a saída à linha de estoque
/// (`CDX_ESTQ → EST_ID`). Os itens de pedido (F05001) não servem: `ITMP_CNF` é a configuração
/// comercial e `ITMP_ESTM` aponta para a linha do produto LISO reservado — outro item.
///
/// Sinal: `VENDA` sai do estoque (`CDX_QTD` negativo) e `DEVOLUCAO_VENDA` entra (positivo); logo
/// o líquido vendido é `-SUM(CDX_QTD)`. O `HAVING` descarta o dia cujo líquido não é positivo
/// (devolução ≥ venda): `vendas_dia` exige `qtd_vendida >= 0` e um dia sem venda líquida não é
/// venda. `$1` = data inicial da janela.
const SQL_VENDAS: &str = "\
SELECT c.cdx_datc::date AS cdx_datc, c.cdx_estq AS cdx_estq, \
       p.itm_sku AS itm_sku, p.itm_desc AS itm_desc, e.est_dconf AS est_dconf, \
       ROUND(-SUM(c.cdx_qtd))::int AS cdx_qtd, \
       BOOL_OR(COALESCE(p.itm_proda, false)) AS itm_proda \
FROM prd.f03007 c \
JOIN prd.f03005 e ON e.est_id = c.cdx_estq \
JOIN prd.f03001 p ON p.itm_id = e.est_itm \
WHERE c.cdx_tpmvm IN ('VENDA', 'DEVOLUCAO_VENDA') \
  AND p.itm_gpprd = 'PRODUTO_ACABADO' AND c.cdx_datc >= $1 \
GROUP BY c.cdx_datc::date, c.cdx_estq, p.itm_sku, p.itm_desc, e.est_dconf \
HAVING ROUND(-SUM(c.cdx_qtd))::int > 0";

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

    /// Lê o estoque cru do One. Falha (não silencia) se uma coluna esperada não decodificar —
    /// quantidade de estoque errada/zerada por engano afeta a recomendação (§7 integridade).
    async fn estoque_cru(&self) -> Result<Vec<BronzeEstoque>, ErroEtl> {
        let linhas = sqlx::query(SQL_ESTOQUE).fetch_all(&self.one).await?;
        linhas
            .iter()
            .map(|r| {
                Ok(BronzeEstoque {
                    est_id: r.try_get("est_id")?,
                    est_itm: r.try_get("est_itm")?,
                    est_cnf: r.try_get::<Option<i64>, _>("est_cnf")?,
                    est_dconf: texto(r, "est_dconf")?,
                    itm_sku: texto(r, "itm_sku")?,
                    itm_desc: texto(r, "itm_desc")?,
                    est_qtde: inteiro(r, "est_qtde")?,
                    est_qtdd: inteiro(r, "est_qtdd")?,
                    est_qtem: r.try_get::<Option<i32>, _>("est_qtem")?,
                    est_flin: booleano(r, "est_flin")?,
                    itm_proda: booleano(r, "itm_proda")?,
                })
            })
            .collect()
    }

    /// Lê as vendas cruas do One a partir de `desde`. Falha em erro de coluna (não silencia).
    async fn vendas_cru(&self, desde: NaiveDate) -> Result<Vec<BronzeVenda>, ErroEtl> {
        let linhas = sqlx::query(SQL_VENDAS)
            .bind(desde)
            .fetch_all(&self.one)
            .await?;
        linhas
            .iter()
            .map(|r| {
                Ok(BronzeVenda {
                    cdx_datc: r.try_get("cdx_datc")?,
                    cdx_estq: r.try_get("cdx_estq")?,
                    itm_sku: texto(r, "itm_sku")?,
                    itm_desc: texto(r, "itm_desc")?,
                    est_dconf: texto(r, "est_dconf")?,
                    cdx_qtd: inteiro(r, "cdx_qtd")?,
                    itm_proda: booleano(r, "itm_proda")?,
                })
            })
            .collect()
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
                 (data_ref, est_id, est_itm, est_cnf, est_dconf, itm_sku, itm_desc, \
                  est_qtde, est_qtdd, est_qtem, est_flin, itm_proda) ",
            );
            qb.push_values(lote, |mut b, r| {
                b.push_bind(data_ref)
                    .push_bind(r.est_id)
                    .push_bind(r.est_itm)
                    .push_bind(r.est_cnf)
                    .push_bind(r.est_dconf.as_deref())
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
        sqlx::query("DELETE FROM bronze.one_venda WHERE cdx_datc >= $1")
            .bind(desde)
            .execute(&mut *tx)
            .await?;
        for lote in cru.chunks(LOTE) {
            let mut qb = QueryBuilder::new(
                "INSERT INTO bronze.one_venda \
                 (cdx_datc, cdx_estq, itm_sku, itm_desc, est_dconf, cdx_qtd, itm_proda) ",
            );
            qb.push_values(lote, |mut b, r| {
                b.push_bind(r.cdx_datc)
                    .push_bind(r.cdx_estq)
                    .push_bind(r.itm_sku.as_deref())
                    .push_bind(r.itm_desc.as_deref())
                    .push_bind(r.est_dconf.as_deref())
                    .push_bind(r.cdx_qtd)
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

/// Lê uma coluna textual opcional, normalizando branco → `None`. Erro de decodificação propaga
/// (não é silenciado): só `NULL` vira `None`.
fn texto(r: &PgRow, col: &str) -> Result<Option<String>, ErroEtl> {
    Ok(r.try_get::<Option<String>, _>(col)?
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty()))
}

/// Lê uma quantidade inteira agregada; `NULL` (SUM de tudo nulo) vira `0`, erro de tipo propaga.
fn inteiro(r: &PgRow, col: &str) -> Result<i32, ErroEtl> {
    Ok(r.try_get::<Option<i32>, _>(col)?.unwrap_or(0))
}

/// Lê um booleano agregado; `NULL` (`BOOL_OR` vazio) vira `false`, erro de tipo propaga.
fn booleano(r: &PgRow, col: &str) -> Result<bool, ErroEtl> {
    Ok(r.try_get::<Option<bool>, _>(col)?.unwrap_or(false))
}
