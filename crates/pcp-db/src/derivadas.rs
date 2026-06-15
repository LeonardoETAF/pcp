//! Persistência das tabelas derivadas e do log de execuções (doc 04 §3 / doc 05 §3).
//! Estruturas de entrada com primitivos: o mapeamento dos tipos de domínio é do `pcp-engine`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::erro::ErroDb;

/// Linha a persistir em `pcp.classificacao` (doc 04 §3.1).
#[derive(Debug, Clone)]
pub struct LinhaClassificacao {
    pub codigo: String,
    pub classe: String,
    pub volume_janela: i64,
    pub percentual_acumulado: Option<f64>,
    pub fator_estoque: f64,
}

/// Linha a persistir em `pcp.estoque_param` (doc 04 §3.2).
#[derive(Debug, Clone)]
pub struct LinhaParametro {
    pub codigo: String,
    pub media_diaria: f64,
    pub desvio: f64,
    pub coef_variacao: f64,
    pub dias_com_vendas: i64,
    pub outliers_detectados: i64,
    pub estoque_minimo: i64,
    pub estoque_seguranca: i64,
    pub estoque_total_recomendado: i64,
    pub sem_historico_confiavel: bool,
    pub fator_sazonal: f64,
}

/// Linha a persistir em `pcp.alerta` (doc 04 §3.3).
#[derive(Debug, Clone)]
pub struct LinhaAlerta {
    pub codigo: String,
    pub prioridade: String,
    pub classe: String,
    pub qtd_sugerida: i64,
    pub cobertura_dias: f64,
}

/// Linha a persistir em `pcp.sugestao_ciclo_vida` (doc 04 §3.4). Nasce no estado `gerada`.
#[derive(Debug, Clone)]
pub struct LinhaSugestao {
    pub codigo: String,
    pub acao: String,
    pub pontuacao: i16,
    pub nivel_certeza: String,
    pub criterios: Vec<String>,
}

/// Linha a persistir em `pcp.produto_ativo` (doc 04 §5) — tudo já calculado pelo motor.
#[derive(Debug, Clone)]
pub struct LinhaProdutoAtivo {
    pub codigo: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub classe: String,
    pub fator_estoque: f64,
    pub qtd_estoque: i64,
    pub qtd_reserva: i64,
    pub qtd_disponivel: i64,
    pub media_diaria: f64,
    pub coef_variacao: f64,
    pub dias_com_vendas: i64,
    pub estoque_minimo: i64,
    pub estoque_seguranca: i64,
    pub estoque_total_recomendado: i64,
    pub cobertura_dias: f64,
    pub status: String,
    pub qtd_sugerida: i64,
    pub fora_de_linha: bool,
    pub volume_janela: i64,
}

/// Telemetria de um módulo do pipeline (doc 05 §3).
#[derive(Debug, Clone)]
pub struct ExecucaoModulo {
    pub modulo: String,
    pub status: String,
    pub linhas: i64,
    pub duracao_ms: i64,
    pub erro: Option<String>,
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
}

/// Regrava a classificação de `dt_calculo` (delete + insert, idempotente — CLAUDE.md §3.3).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco; a transação é revertida.
pub async fn salvar_classificacao(
    pool: &PgPool,
    dt_calculo: NaiveDate,
    linhas: &[LinhaClassificacao],
) -> Result<u64, ErroDb> {
    let mut tx = pool.begin().await?;
    sqlx::query!(
        "DELETE FROM pcp.classificacao WHERE dt_calculo = $1",
        dt_calculo
    )
    .execute(&mut *tx)
    .await?;
    let mut inseridas = 0;
    for l in linhas {
        sqlx::query!(
            "INSERT INTO pcp.classificacao \
             (dt_calculo, codigo_estoque, classe, volume_janela, percentual_acumulado, fator_estoque) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            dt_calculo,
            l.codigo,
            l.classe,
            l.volume_janela,
            l.percentual_acumulado,
            l.fator_estoque,
        )
        .execute(&mut *tx)
        .await?;
        inseridas += 1;
    }
    tx.commit().await?;
    Ok(inseridas)
}

/// Upsert dos parâmetros de estoque por produto (estado atual — doc 04 §3.2).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco; a transação é revertida.
pub async fn salvar_parametros(
    pool: &PgPool,
    dt_calc: NaiveDate,
    linhas: &[LinhaParametro],
) -> Result<u64, ErroDb> {
    let mut tx = pool.begin().await?;
    let mut afetadas = 0;
    for l in linhas {
        sqlx::query!(
            "INSERT INTO pcp.estoque_param \
             (codigo_estoque, media_diaria, desvio, coef_variacao, dias_com_vendas, \
              outliers_detectados, estoque_minimo, estoque_seguranca, estoque_total_recomendado, \
              sem_historico_confiavel, fator_sazonal, dt_calc, atualizado_em) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now()) \
             ON CONFLICT (codigo_estoque) DO UPDATE SET \
              media_diaria = EXCLUDED.media_diaria, desvio = EXCLUDED.desvio, \
              coef_variacao = EXCLUDED.coef_variacao, dias_com_vendas = EXCLUDED.dias_com_vendas, \
              outliers_detectados = EXCLUDED.outliers_detectados, \
              estoque_minimo = EXCLUDED.estoque_minimo, estoque_seguranca = EXCLUDED.estoque_seguranca, \
              estoque_total_recomendado = EXCLUDED.estoque_total_recomendado, \
              sem_historico_confiavel = EXCLUDED.sem_historico_confiavel, \
              fator_sazonal = EXCLUDED.fator_sazonal, dt_calc = EXCLUDED.dt_calc, atualizado_em = now()",
            l.codigo,
            l.media_diaria,
            l.desvio,
            l.coef_variacao,
            l.dias_com_vendas,
            l.outliers_detectados,
            l.estoque_minimo,
            l.estoque_seguranca,
            l.estoque_total_recomendado,
            l.sem_historico_confiavel,
            l.fator_sazonal,
            dt_calc,
        )
        .execute(&mut *tx)
        .await?;
        afetadas += 1;
    }
    tx.commit().await?;
    Ok(afetadas)
}

/// Regrava os alertas de `dt_alerta` (delete + insert, idempotente — doc 02 §6).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco; a transação é revertida.
pub async fn salvar_alertas(
    pool: &PgPool,
    dt_alerta: NaiveDate,
    linhas: &[LinhaAlerta],
) -> Result<u64, ErroDb> {
    let mut tx = pool.begin().await?;
    sqlx::query!("DELETE FROM pcp.alerta WHERE dt_alerta = $1", dt_alerta)
        .execute(&mut *tx)
        .await?;
    let mut inseridos = 0;
    for l in linhas {
        sqlx::query!(
            "INSERT INTO pcp.alerta \
             (dt_alerta, codigo_estoque, prioridade, classe, qtd_sugerida, cobertura_dias) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            dt_alerta,
            l.codigo,
            l.prioridade,
            l.classe,
            l.qtd_sugerida,
            l.cobertura_dias,
        )
        .execute(&mut *tx)
        .await?;
        inseridos += 1;
    }
    tx.commit().await?;
    Ok(inseridos)
}

/// Persiste as sugestões de ciclo de vida do dia: apaga as `gerada` da data e insere as
/// novas, sem duplicar sugestões abertas (índice parcial único — doc 04 §3.4).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco; a transação é revertida.
pub async fn salvar_sugestoes(
    pool: &PgPool,
    data_analise: NaiveDate,
    linhas: &[LinhaSugestao],
) -> Result<u64, ErroDb> {
    let mut tx = pool.begin().await?;
    sqlx::query!(
        "DELETE FROM pcp.sugestao_ciclo_vida WHERE data_analise = $1 AND estado = 'gerada'",
        data_analise,
    )
    .execute(&mut *tx)
    .await?;
    let mut inseridas = 0;
    for l in linhas {
        let resultado = sqlx::query!(
            "INSERT INTO pcp.sugestao_ciclo_vida \
             (codigo_estoque, acao_sugerida, pontuacao, nivel_certeza, criterios, estado, data_analise) \
             VALUES ($1, $2, $3, $4, $5, 'gerada', $6) \
             ON CONFLICT DO NOTHING",
            l.codigo,
            l.acao,
            l.pontuacao,
            l.nivel_certeza,
            &l.criterios,
            data_analise,
        )
        .execute(&mut *tx)
        .await?;
        inseridas += resultado.rows_affected();
    }
    tx.commit().await?;
    Ok(inseridas)
}

/// Reescreve por completo a "view" materializada `pcp.produto_ativo` (doc 04 §5):
/// `TRUNCATE` + insert na mesma transação. A API lê só daqui (CLAUDE.md §3.2).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco; a transação é revertida.
pub async fn salvar_produtos_ativos(
    pool: &PgPool,
    dt_ref: NaiveDate,
    linhas: &[LinhaProdutoAtivo],
) -> Result<u64, ErroDb> {
    let mut tx = pool.begin().await?;
    sqlx::query!("TRUNCATE pcp.produto_ativo")
        .execute(&mut *tx)
        .await?;
    let mut inseridas = 0;
    for l in linhas {
        sqlx::query!(
            "INSERT INTO pcp.produto_ativo \
             (codigo_estoque, sku, produto, configuracao, classe, fator_estoque, \
              qtd_estoque, qtd_reserva, qtd_disponivel, media_diaria, coef_variacao, \
              dias_com_vendas, estoque_minimo, estoque_seguranca, estoque_total_recomendado, \
              cobertura_dias, status, qtd_sugerida, fora_de_linha, volume_janela, dt_ref) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, \
                     $16, $17, $18, $19, $20, $21)",
            l.codigo,
            l.sku,
            l.produto,
            l.configuracao,
            l.classe,
            l.fator_estoque,
            l.qtd_estoque,
            l.qtd_reserva,
            l.qtd_disponivel,
            l.media_diaria,
            l.coef_variacao,
            l.dias_com_vendas,
            l.estoque_minimo,
            l.estoque_seguranca,
            l.estoque_total_recomendado,
            l.cobertura_dias,
            l.status,
            l.qtd_sugerida,
            l.fora_de_linha,
            l.volume_janela,
            dt_ref,
        )
        .execute(&mut *tx)
        .await?;
        inseridas += 1;
    }
    tx.commit().await?;
    Ok(inseridas)
}

/// Registra a telemetria de um módulo do pipeline (doc 05 §3).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn registrar_execucao(
    pool: &PgPool,
    data_ref: NaiveDate,
    execucao: &ExecucaoModulo,
) -> Result<(), ErroDb> {
    sqlx::query!(
        "INSERT INTO pcp.execucao_pipeline \
         (data_ref, modulo, status, linhas_afetadas, duracao_ms, erro, inicio, fim) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        data_ref,
        execucao.modulo,
        execucao.status,
        execucao.linhas,
        execucao.duracao_ms,
        execucao.erro,
        execucao.inicio,
        execucao.fim,
    )
    .execute(pool)
    .await?;
    Ok(())
}
