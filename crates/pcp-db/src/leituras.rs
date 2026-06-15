//! Consultas de LEITURA da API (doc 04 §6.2). Leem só a "view" materializada `produto_ativo`
//! e os alertas do dia — valores já calculados pelo motor; **nenhuma regra é recalculada aqui**
//! (CLAUDE.md §3.2). Agregações pesadas ficam no banco (§15). A cobertura sentinela 999 nunca
//! entra em médias (§11).

use sqlx::PgPool;

use crate::erro::ErroDb;

/// Cobertura sentinela (produto sem histórico) — espelha `pcp_core::COBERTURA_SEM_HISTORICO`.
/// Usada só para EXCLUIR esses produtos das médias no SQL (§11); não é regra de negócio.
const COBERTURA_SEM_HISTORICO: f64 = 999.0;

/// Contagem `(rótulo, quantidade)` para distribuições por classe/status.
#[derive(Debug, Clone)]
pub struct Contagem {
    pub rotulo: String,
    pub quantidade: i64,
}

/// Métricas agregadas do dashboard (doc 04 §6.2 — `get_dashboard_completo`).
#[derive(Debug, Clone)]
pub struct ResumoDashboard {
    pub data_ref: Option<chrono::NaiveDate>,
    pub total_produtos: i64,
    pub total_sugerido: i64,
    pub cobertura_media: Option<f64>,
    pub por_classe: Vec<Contagem>,
    pub por_status: Vec<Contagem>,
}

/// Linha da tabela de estoque paginada (doc 04 §6.2 — `get_produtos_ativos_paginado`).
#[derive(Debug, Clone)]
pub struct LinhaEstoque {
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub classe: String,
    pub qtd_disponivel: i64,
    pub cobertura_dias: f64,
    pub estoque_total_recomendado: i64,
    pub status: String,
    pub qtd_sugerida: i64,
    pub fora_de_linha: bool,
}

/// Página de produtos com o total que satisfaz o filtro (paginação no servidor — §15).
#[derive(Debug, Clone)]
pub struct PaginaEstoque {
    pub itens: Vec<LinhaEstoque>,
    pub total: i64,
}

/// Distribuição por classe ABC (doc 04 §6.2 — `get_distribuicao_abc_estoque`).
#[derive(Debug, Clone)]
pub struct DistribuicaoClasse {
    pub classe: String,
    pub quantidade: i64,
    pub volume: i64,
    pub recomendado: i64,
}

/// Alerta enriquecido para a Central de Alertas (doc 04 §6.2 — `get_alertas_completos`).
/// O nome de exibição (`produto - cor`) é montado no frontend a partir de `configuracao` (§12).
#[derive(Debug, Clone)]
pub struct AlertaCompleto {
    pub codigo_estoque: String,
    pub prioridade: String,
    pub classe: String,
    pub qtd_sugerida: i64,
    pub cobertura_dias: f64,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub status: Option<String>,
}

/// Alertas do dia mais recente, enriquecidos com dados do produto (doc 04 §6.2).
/// Ordem: prioridade (crítico→alto→médio) e maior sugestão primeiro.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn alertas_do_dia(pool: &PgPool) -> Result<Vec<AlertaCompleto>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT a.codigo_estoque AS "codigo_estoque!",
                  a.prioridade     AS "prioridade!",
                  a.classe         AS "classe!",
                  a.qtd_sugerida   AS "qtd_sugerida!",
                  a.cobertura_dias AS "cobertura_dias!",
                  p.produto        AS "produto?",
                  p.configuracao   AS "configuracao?",
                  p.status         AS "status?"
           FROM pcp.alerta a
           LEFT JOIN pcp.produto_ativo p ON p.codigo_estoque = a.codigo_estoque
           WHERE a.dt_alerta = (SELECT MAX(dt_alerta) FROM pcp.alerta)
           ORDER BY CASE a.prioridade
                        WHEN 'critico' THEN 0 WHEN 'alto' THEN 1 ELSE 2 END,
                    a.qtd_sugerida DESC, a.codigo_estoque"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| AlertaCompleto {
            codigo_estoque: r.codigo_estoque,
            prioridade: r.prioridade,
            classe: r.classe,
            qtd_sugerida: r.qtd_sugerida,
            cobertura_dias: r.cobertura_dias,
            produto: r.produto,
            configuracao: r.configuracao,
            status: r.status,
        })
        .collect())
}

/// Métricas do dashboard a partir de `produto_ativo` (uma passada agregada — §15).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn dashboard(pool: &PgPool) -> Result<ResumoDashboard, ErroDb> {
    let totais = sqlx::query!(
        r#"SELECT MAX(dt_ref)                                            AS "data_ref?",
                  COUNT(*)                                               AS "total!",
                  COALESCE(SUM(qtd_sugerida), 0)::bigint                 AS "sugerido!",
                  AVG(cobertura_dias) FILTER (WHERE cobertura_dias <> $1) AS "cobertura?"
           FROM pcp.produto_ativo"#,
        COBERTURA_SEM_HISTORICO,
    )
    .fetch_one(pool)
    .await?;

    let por_classe = sqlx::query!(
        r#"SELECT classe AS "rotulo!", COUNT(*) AS "quantidade!"
           FROM pcp.produto_ativo GROUP BY classe ORDER BY classe"#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| Contagem {
        rotulo: r.rotulo,
        quantidade: r.quantidade,
    })
    .collect();

    let por_status = sqlx::query!(
        r#"SELECT status AS "rotulo!", COUNT(*) AS "quantidade!"
           FROM pcp.produto_ativo GROUP BY status ORDER BY status"#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| Contagem {
        rotulo: r.rotulo,
        quantidade: r.quantidade,
    })
    .collect();

    Ok(ResumoDashboard {
        data_ref: totais.data_ref,
        total_produtos: totais.total,
        total_sugerido: totais.sugerido,
        cobertura_media: totais.cobertura,
        por_classe,
        por_status,
    })
}

/// Produtos ativos paginados (doc 04 §6.2). Filtros opcionais por `classe`/`status`; ordenação
/// por sugestão decrescente (mais urgentes primeiro). Total calculado no mesmo filtro.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn produtos_paginado(
    pool: &PgPool,
    classe: Option<&str>,
    status: Option<&str>,
    limite: i64,
    deslocamento: i64,
) -> Result<PaginaEstoque, ErroDb> {
    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "total!" FROM pcp.produto_ativo
           WHERE ($1::text IS NULL OR classe = $1)
             AND ($2::text IS NULL OR status = $2)"#,
        classe,
        status,
    )
    .fetch_one(pool)
    .await?;

    let itens = sqlx::query!(
        r#"SELECT codigo_estoque, sku, produto, configuracao, classe,
                  qtd_disponivel, cobertura_dias, estoque_total_recomendado,
                  status, qtd_sugerida, fora_de_linha
           FROM pcp.produto_ativo
           WHERE ($1::text IS NULL OR classe = $1)
             AND ($2::text IS NULL OR status = $2)
           ORDER BY qtd_sugerida DESC, codigo_estoque
           LIMIT $3 OFFSET $4"#,
        classe,
        status,
        limite,
        deslocamento,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| LinhaEstoque {
        codigo_estoque: r.codigo_estoque,
        sku: r.sku,
        produto: r.produto,
        configuracao: r.configuracao,
        classe: r.classe,
        qtd_disponivel: r.qtd_disponivel,
        cobertura_dias: r.cobertura_dias,
        estoque_total_recomendado: r.estoque_total_recomendado,
        status: r.status,
        qtd_sugerida: r.qtd_sugerida,
        fora_de_linha: r.fora_de_linha,
    })
    .collect();

    Ok(PaginaEstoque { itens, total })
}

/// Distribuição por classe ABC: contagem, volume e recomendado somados (doc 04 §6.2).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn distribuicao_abc(pool: &PgPool) -> Result<Vec<DistribuicaoClasse>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT classe                                       AS "classe!",
                  COUNT(*)                                     AS "quantidade!",
                  COALESCE(SUM(volume_janela), 0)::bigint      AS "volume!",
                  COALESCE(SUM(estoque_total_recomendado), 0)::bigint AS "recomendado!"
           FROM pcp.produto_ativo GROUP BY classe ORDER BY classe"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| DistribuicaoClasse {
            classe: r.classe,
            quantidade: r.quantidade,
            volume: r.volume,
            recomendado: r.recomendado,
        })
        .collect())
}
