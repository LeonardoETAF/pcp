//! Orquestração da sazonalidade (doc 02 §4): decide o gatilho, agrega as vendas do ano
//! anterior (via pcp-db), calcula os 12 fatores (via pcp-core) e os persiste. O recálculo é
//! FAILSAFE: erro não derruba o pipeline, mantém os fatores anteriores (§4.2). Log via tracing.
// Médias diárias: totais/dias pequenos cabem exatos em f64.
#![allow(clippy::cast_precision_loss)]

use chrono::{Datelike, NaiveDate};

use std::collections::HashMap;

use pcp_core::sazonalidade::{
    calcular_fator, calcular_fatores_produto, deve_recalcular, ParametrosSazonalidade,
    VendasMesProduto,
};
use pcp_db::{sazonalidade as db, PgPool};

use crate::erro::ErroEngine;

/// Desfecho de uma tentativa de atualização dos fatores sazonais.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoSazonalidade {
    /// Fatores recalculados e persistidos.
    Recalculado,
    /// Gatilho não disparou (fatores ainda atuais).
    NaoNecessario,
    /// Recálculo falhou; fatores anteriores mantidos (failsafe — doc 02 §4.2).
    FalhaIgnorada,
}

/// Atualiza os fatores sazonais se o gatilho disparar (doc 02 §4.2), de forma failsafe.
///
/// # Errors
/// [`ErroEngine::Db`] apenas na leitura inicial do gatilho. Falhas do recálculo em si NÃO
/// são propagadas (são logadas e resultam em [`ResultadoSazonalidade::FalhaIgnorada`]).
pub async fn atualizar_fatores(
    pool: &PgPool,
    hoje: NaiveDate,
    params: ParametrosSazonalidade,
) -> Result<ResultadoSazonalidade, ErroEngine> {
    let ultima = db::ultima_atualizacao(pool).await?;
    // Sem NENHUMA curva por produto, recalcula mesmo que o gatilho mensal não tenha disparado —
    // é o primeiro cálculo depois de ligar a sazonalidade por produto (doc 02 §4.2).
    let sem_curvas = !db::tem_curvas_por_produto(pool).await?;
    if !sem_curvas && !deve_recalcular(ultima, hoje, params.atualizar_apos_dias) {
        tracing::debug!("sazonalidade: recálculo não necessário");
        return Ok(ResultadoSazonalidade::NaoNecessario);
    }
    match recalcular(pool, hoje, params).await {
        Ok(()) => {
            tracing::info!(
                ano_base = hoje.year() - 1,
                "sazonalidade: fatores recalculados"
            );
            Ok(ResultadoSazonalidade::Recalculado)
        }
        Err(erro) => {
            tracing::error!(%erro, "sazonalidade: falha no recálculo; mantidos os fatores anteriores");
            Ok(ResultadoSazonalidade::FalhaIgnorada)
        }
    }
}

/// Recalcula e persiste os 12 fatores a partir das vendas do ano anterior (doc 02 §4.1).
async fn recalcular(
    pool: &PgPool,
    hoje: NaiveDate,
    params: ParametrosSazonalidade,
) -> Result<(), ErroEngine> {
    let ano_anterior = hoje.year() - 1;
    let inicio = NaiveDate::from_ymd_opt(ano_anterior, 1, 1).ok_or(ErroEngine::DataInvalida)?;
    let fim = NaiveDate::from_ymd_opt(ano_anterior + 1, 1, 1).ok_or(ErroEngine::DataInvalida)?;

    let vendas = db::vendas_por_mes(pool, inicio, fim).await?;
    let total_ano: f64 = vendas.iter().map(|v| v.total).sum();
    let dias_ano: i64 = vendas.iter().map(|v| v.dias).sum();
    let media_ano = if dias_ano > 0 {
        total_ano / dias_ano as f64
    } else {
        0.0
    };

    // Meses sem venda no ano anterior ficam neutros (fator 1.0).
    let mut fatores = [1.0_f64; 12];
    for v in &vendas {
        if (1..=12).contains(&v.mes) && v.dias > 0 {
            let media_mes = v.total / v.dias as f64;
            let indice = usize::try_from(v.mes - 1).unwrap_or(0);
            fatores[indice] =
                calcular_fator(media_mes, media_ano, params.clamp_min, params.clamp_max);
        }
    }

    db::substituir(pool, &fatores).await?;
    recalcular_por_produto(pool, inicio, fim, params).await?;
    Ok(())
}

/// Curva sazonal PRÓPRIA de cada produto (doc 02 §4, por produto — decisão do dono, 2026-07-13).
/// Produto sem histórico suficiente fica FORA da tabela: o pipeline cai no fator global.
async fn recalcular_por_produto(
    pool: &PgPool,
    inicio: NaiveDate,
    fim: NaiveDate,
    params: ParametrosSazonalidade,
) -> Result<(), ErroEngine> {
    let vendas = db::vendas_por_produto_mes(pool, inicio, fim).await?;

    let mut por_produto: HashMap<String, Vec<VendasMesProduto>> = HashMap::new();
    for v in vendas {
        let mes = u32::try_from(v.mes).unwrap_or(0);
        por_produto
            .entry(v.codigo_estoque)
            .or_default()
            .push(VendasMesProduto {
                mes,
                total: v.total,
                dias: v.dias,
            });
    }

    let total_produtos = por_produto.len();
    let fatores: Vec<(String, [f64; 12])> = por_produto
        .into_iter()
        .filter_map(|(codigo, meses)| {
            calcular_fatores_produto(
                &meses,
                params.min_meses_com_venda_produto,
                params.clamp_min,
                params.clamp_max,
            )
            .map(|f| (codigo, f))
        })
        .collect();

    let com_curva = fatores.len();
    db::substituir_por_produto(pool, &fatores).await?;
    tracing::info!(
        com_curva,
        sem_curva = total_produtos.saturating_sub(com_curva),
        min_meses = params.min_meses_com_venda_produto,
        "sazonalidade: curvas por produto recalculadas (o resto usa a curva global)"
    );
    Ok(())
}
