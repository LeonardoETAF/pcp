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

/// Linha da tabela de estoque paginada (doc 04 §6.2 / doc 03 §3.3).
#[derive(Debug, Clone)]
pub struct LinhaEstoque {
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub classe: String,
    pub qtd_estoque: i64,
    pub qtd_reserva: i64,
    pub qtd_disponivel: i64,
    pub media_diaria: f64,
    pub cobertura_dias: f64,
    pub estoque_minimo: i64,
    pub estoque_total_recomendado: i64,
    pub volume_janela: i64,
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

/// Resumo por classe para o dashboard executivo (doc 03 §2): contagem, estoque físico (soma de
/// `qtd_estoque`) e cobertura média (exclui a sentinela 999 — §11).
#[derive(Debug, Clone)]
pub struct ResumoClasse {
    pub classe: String,
    pub qtd_produtos: i64,
    pub estoque_fisico: i64,
    pub cobertura_media: Option<f64>,
}

/// Total de vendas de um mês (série mensal do dashboard, doc 03 §2). Dado REAL de `vendas_dia`.
#[derive(Debug, Clone)]
pub struct VendaMes {
    pub ano: i32,
    pub mes: i32,
    pub total: i64,
}

/// Linha da tabela de Classificação ABC (doc 03 §6): 1 por produto, classificação MAIS RECENTE.
#[derive(Debug, Clone)]
pub struct LinhaAbc {
    pub codigo_estoque: String,
    pub produto: Option<String>,
    pub classe: String,
    pub volume_janela: i64,
    pub percentual_acumulado: Option<f64>,
    pub fator_estoque: f64,
    pub estoque_atual: i64,
    pub status: String,
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

/// Filtros e ordenação da tabela de estoque (doc 03 §3.2). `busca` casa código/produto/SKU
/// (parcial, case-insensitive). `ordem` é uma chave coluna+direção da allowlist (ver SQL).
/// Faixa de cobertura (dias) e switches `apenas_sugestao`/`apenas_fora_linha` afinam a fila.
#[derive(Debug, Clone, Copy, Default)]
pub struct FiltroEstoque<'a> {
    pub classe: Option<&'a str>,
    pub status: Option<&'a str>,
    pub busca: Option<&'a str>,
    pub ordem: &'a str,
    pub cobertura_min: Option<f64>,
    pub cobertura_max: Option<f64>,
    pub apenas_sugestao: bool,
    pub apenas_fora_linha: bool,
}

/// Produtos ativos paginados (doc 03 §3 / doc 04 §6.2). Filtra por classe/status/busca e ordena
/// por uma chave da allowlist (compile-time safe via CASE). Total calculado no mesmo filtro.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn produtos_paginado(
    pool: &PgPool,
    filtro: FiltroEstoque<'_>,
    limite: i64,
    deslocamento: i64,
) -> Result<PaginaEstoque, ErroDb> {
    let FiltroEstoque {
        classe,
        status,
        busca,
        ordem,
        cobertura_min,
        cobertura_max,
        apenas_sugestao,
        apenas_fora_linha,
    } = filtro;
    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "total!" FROM pcp.produto_ativo
           WHERE ($1::text IS NULL OR classe = $1)
             AND ($2::text IS NULL OR status = $2)
             AND ($3::text IS NULL OR codigo_estoque ILIKE '%' || $3 || '%'
                  OR produto ILIKE '%' || $3 || '%' OR sku ILIKE '%' || $3 || '%')
             AND ($4::float8 IS NULL OR cobertura_dias >= $4)
             AND ($5::float8 IS NULL OR cobertura_dias <= $5)
             AND (NOT $6 OR qtd_sugerida > 0)
             AND (NOT $7 OR fora_de_linha)"#,
        classe,
        status,
        busca,
        cobertura_min,
        cobertura_max,
        apenas_sugestao,
        apenas_fora_linha,
    )
    .fetch_one(pool)
    .await?;

    let itens = sqlx::query!(
        r#"SELECT codigo_estoque, sku, produto, configuracao, classe,
                  qtd_estoque, qtd_reserva, qtd_disponivel, media_diaria,
                  cobertura_dias, estoque_minimo, estoque_total_recomendado,
                  volume_janela, status, qtd_sugerida, fora_de_linha
           FROM pcp.produto_ativo
           WHERE ($1::text IS NULL OR classe = $1)
             AND ($2::text IS NULL OR status = $2)
             AND ($3::text IS NULL OR codigo_estoque ILIKE '%' || $3 || '%'
                  OR produto ILIKE '%' || $3 || '%' OR sku ILIKE '%' || $3 || '%')
             AND ($5::float8 IS NULL OR cobertura_dias >= $5)
             AND ($6::float8 IS NULL OR cobertura_dias <= $6)
             AND (NOT $7 OR qtd_sugerida > 0)
             AND (NOT $8 OR fora_de_linha)
           ORDER BY
             (CASE WHEN $4 = 'produto_asc' THEN produto END) ASC NULLS LAST,
             (CASE WHEN $4 = 'produto_desc' THEN produto END) DESC NULLS LAST,
             (CASE WHEN $4 = 'classe_asc' THEN classe END) ASC NULLS LAST,
             (CASE WHEN $4 = 'disponivel_asc' THEN qtd_disponivel END) ASC NULLS LAST,
             (CASE WHEN $4 = 'disponivel_desc' THEN qtd_disponivel END) DESC NULLS LAST,
             (CASE WHEN $4 = 'cobertura_asc' THEN cobertura_dias END) ASC NULLS LAST,
             (CASE WHEN $4 = 'cobertura_desc' THEN cobertura_dias END) DESC NULLS LAST,
             (CASE WHEN $4 = 'recomendada_desc' THEN estoque_total_recomendado END) DESC NULLS LAST,
             (CASE WHEN $4 = 'sugerida_asc' THEN qtd_sugerida END) ASC NULLS LAST,
             (CASE WHEN $4 NOT IN ('produto_asc','produto_desc','classe_asc','disponivel_asc',
                  'disponivel_desc','cobertura_asc','cobertura_desc','recomendada_desc','sugerida_asc')
                THEN qtd_sugerida END) DESC NULLS LAST,
             codigo_estoque
           LIMIT $9 OFFSET $10"#,
        classe,
        status,
        busca,
        ordem,
        cobertura_min,
        cobertura_max,
        apenas_sugestao,
        apenas_fora_linha,
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
        qtd_estoque: r.qtd_estoque,
        qtd_reserva: r.qtd_reserva,
        qtd_disponivel: r.qtd_disponivel,
        media_diaria: r.media_diaria,
        cobertura_dias: r.cobertura_dias,
        estoque_minimo: r.estoque_minimo,
        estoque_total_recomendado: r.estoque_total_recomendado,
        volume_janela: r.volume_janela,
        status: r.status,
        qtd_sugerida: r.qtd_sugerida,
        fora_de_linha: r.fora_de_linha,
    })
    .collect();

    Ok(PaginaEstoque { itens, total })
}

/// Resumo por classe (contagem, estoque físico, cobertura média) — dashboard executivo (§2).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn resumo_por_classe(pool: &PgPool) -> Result<Vec<ResumoClasse>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT classe                                            AS "classe!",
                  COUNT(*)                                          AS "qtd_produtos!",
                  COALESCE(SUM(qtd_estoque), 0)::bigint             AS "estoque_fisico!",
                  AVG(cobertura_dias) FILTER (WHERE cobertura_dias <> $1) AS "cobertura_media?"
           FROM pcp.produto_ativo GROUP BY classe ORDER BY classe"#,
        COBERTURA_SEM_HISTORICO,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| ResumoClasse {
            classe: r.classe,
            qtd_produtos: r.qtd_produtos,
            estoque_fisico: r.estoque_fisico,
            cobertura_media: r.cobertura_media,
        })
        .collect())
}

/// Série mensal de vendas (soma de `qtd_vendida` por mês) — últimos `meses` meses, em ordem
/// cronológica. Dado real de `pcp.vendas_dia` (doc 03 §2). Usado no gráfico de barras do dashboard.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn vendas_mensais(pool: &PgPool, meses: i64) -> Result<Vec<VendaMes>, ErroDb> {
    // Pega os N meses mais recentes (DESC) e reordena em ordem cronológica no Rust.
    let mut linhas = sqlx::query!(
        r#"SELECT EXTRACT(YEAR  FROM dt_ref)::int  AS "ano!",
                  EXTRACT(MONTH FROM dt_ref)::int  AS "mes!",
                  COALESCE(SUM(qtd_vendida), 0)::bigint AS "total!"
           FROM pcp.vendas_dia
           GROUP BY 1, 2
           ORDER BY 1 DESC, 2 DESC
           LIMIT $1"#,
        meses,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| VendaMes {
        ano: r.ano,
        mes: r.mes,
        total: r.total,
    })
    .collect::<Vec<_>>();
    linhas.reverse(); // cronológico (mais antigo → mais recente)
    Ok(linhas)
}

/// Tabela ABC: 1 linha por produto pela classificação MAIS RECENTE (doc 03 §6 — corrige as
/// duplicatas históricas do legado), ordenada por volume desc (ordem de Pareto).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn classificacao_recente(pool: &PgPool) -> Result<Vec<LinhaAbc>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT c.codigo_estoque                       AS "codigo_estoque!",
                  p.produto                              AS "produto?",
                  c.classe                               AS "classe!",
                  c.volume_janela                        AS "volume_janela!",
                  c.percentual_acumulado                 AS "percentual_acumulado?",
                  c.fator_estoque                        AS "fator_estoque!",
                  COALESCE(p.qtd_disponivel, 0)::bigint  AS "estoque_atual!",
                  COALESCE(p.status, 'sem_historico')    AS "status!"
           FROM pcp.classificacao c
           LEFT JOIN pcp.produto_ativo p ON p.codigo_estoque = c.codigo_estoque
           WHERE c.dt_calculo = (SELECT MAX(dt_calculo) FROM pcp.classificacao)
           ORDER BY c.volume_janela DESC, c.codigo_estoque"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| LinhaAbc {
            codigo_estoque: r.codigo_estoque,
            produto: r.produto,
            classe: r.classe,
            volume_janela: r.volume_janela,
            percentual_acumulado: r.percentual_acumulado,
            fator_estoque: r.fator_estoque,
            estoque_atual: r.estoque_atual,
            status: r.status,
        })
        .collect())
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
