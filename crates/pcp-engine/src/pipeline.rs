//! Orquestrador diário do PCP (doc 05 §1.2): pré-validação bloqueante → sazonalidade →
//! classificação → parâmetros → alertas → fora de linha. Idempotente por `data_ref`, com
//! isolamento de falha e telemetria por módulo (doc 05 §3). A regra vive no `pcp-core`.

use std::collections::HashMap;
use std::future::Future;

use chrono::{Datelike, Duration, NaiveDate, Utc};

use pcp_config::Config;
use pcp_core::ciclo_vida::analisar;
use pcp_core::sazonalidade::FatoresSazonais;
use pcp_core::{
    calcular_parametros, classificar, gerar_alertas, AcaoSugerida, ClasseAbc, CodigoEstoque,
    EntradaAlerta, EntradaCicloVida, NivelCerteza, ParametrosEstoque, Prioridade,
    ProdutoParaClassificar, ResultadoClassificacao, StatusParametros,
};
use pcp_db::agregacoes::{self, BaseProduto};
use pcp_db::derivadas::{
    self, ExecucaoModulo, LinhaAlerta, LinhaClassificacao, LinhaParametro, LinhaSugestao,
};
use pcp_db::{sazonalidade as db_sazon, PgPool};

use crate::erro::ErroEngine;
use crate::{mapeamento, sazonalidade};

/// Desfecho do processamento de uma data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusPipeline {
    /// Pré-validação falhou; nada foi processado (doc 05 §3).
    Bloqueado,
    /// Todos os módulos tiveram sucesso.
    Completo,
    /// Pelo menos um módulo falhou (resultado parcial — doc 05 §1.2).
    Parcial,
}

/// Resultado do pipeline de uma data, com a telemetria por módulo.
#[derive(Debug)]
pub struct ResultadoPipeline {
    pub data_ref: NaiveDate,
    pub status: StatusPipeline,
    pub execucoes: Vec<ExecucaoModulo>,
}

/// Processa (ou reprocessa) uma `data_ref` de forma idempotente (doc 05 §1.2).
///
/// # Errors
/// [`ErroEngine`] em falha de infraestrutura na pré-validação ou no carregamento da base.
/// Falhas DENTRO de um módulo não são propagadas: viram telemetria e status parcial.
pub async fn processar_dia(
    pool: &PgPool,
    config: &Config,
    data_ref: NaiveDate,
) -> Result<ResultadoPipeline, ErroEngine> {
    // Pré-validação bloqueante (doc 05 §3): vendas do dia anterior e snapshot do dia.
    let vendas_anterior = agregacoes::contar_vendas(pool, data_ref - Duration::days(1)).await?;
    let snapshot_atual = agregacoes::contar_snapshot(pool, data_ref).await?;
    if vendas_anterior == 0 || snapshot_atual == 0 {
        tracing::warn!(%data_ref, vendas_anterior, snapshot_atual, "pré-validação falhou; pipeline bloqueado");
        return Ok(ResultadoPipeline {
            data_ref,
            status: StatusPipeline::Bloqueado,
            execucoes: Vec::new(),
        });
    }

    // Sazonalidade (failsafe — doc 02 §4.2) e fator do mês corrente.
    sazonalidade::atualizar_fatores(pool, data_ref, mapeamento::parametros_sazonalidade(config))
        .await?;
    let fatores = carregar_fatores(pool).await?;
    let fator_mes = fatores.obter_fator(data_ref.month());

    let base = agregacoes::base_produtos(
        pool,
        data_ref,
        data_ref - Duration::days(540),
        data_ref - Duration::days(365),
    )
    .await?;

    let mut execucoes = Vec::with_capacity(4);

    // 1. Classificação → fonte das classes para os demais módulos.
    let (exec, classes_res) = executar(
        pool,
        data_ref,
        "classificacao",
        modulo_classificacao(pool, &base, config, data_ref),
    )
    .await;
    execucoes.push(exec);
    let classes: HashMap<String, ClasseAbc> = classes_res
        .map(|rs| {
            rs.into_iter()
                .map(|r| (r.codigo_estoque.como_str().to_owned(), r.classe))
                .collect()
        })
        .unwrap_or_default();

    // 2. Parâmetros estatísticos.
    let (exec, params_res) = executar(
        pool,
        data_ref,
        "parametros",
        modulo_parametros(pool, &base, &classes, config, fator_mes, data_ref),
    )
    .await;
    execucoes.push(exec);
    let params = params_res.unwrap_or_default();

    // 3. Alertas de produção.
    let (exec, _) = executar(
        pool,
        data_ref,
        "alertas",
        modulo_alertas(pool, &base, &classes, &params, config, data_ref),
    )
    .await;
    execucoes.push(exec);

    // 4. Análise de fora de linha.
    let (exec, _) = executar(
        pool,
        data_ref,
        "fora_de_linha",
        modulo_fora_de_linha(pool, &base, &classes, config, data_ref),
    )
    .await;
    execucoes.push(exec);

    let status = if execucoes.iter().all(|e| e.status == "sucesso") {
        StatusPipeline::Completo
    } else {
        StatusPipeline::Parcial
    };
    Ok(ResultadoPipeline {
        data_ref,
        status,
        execucoes,
    })
}

/// Reprocessa um intervalo `[inicio, fim]` (inclusive), dia a dia (idempotente).
///
/// # Errors
/// [`ErroEngine`] na primeira data que falhar na infraestrutura.
pub async fn reprocessar_intervalo(
    pool: &PgPool,
    config: &Config,
    inicio: NaiveDate,
    fim: NaiveDate,
) -> Result<Vec<ResultadoPipeline>, ErroEngine> {
    let mut resultados = Vec::new();
    let mut dia = inicio;
    while dia <= fim {
        resultados.push(processar_dia(pool, config, dia).await?);
        dia += Duration::days(1);
    }
    Ok(resultados)
}

/// Executa um módulo medindo tempo, isolando falha e registrando a telemetria (doc 05 §3).
async fn executar<T, Fut>(
    pool: &PgPool,
    data_ref: NaiveDate,
    modulo: &str,
    trabalho: Fut,
) -> (ExecucaoModulo, Option<T>)
where
    Fut: Future<Output = Result<(T, u64), ErroEngine>>,
{
    let inicio = Utc::now();
    let resultado = trabalho.await;
    let fim = Utc::now();
    let duracao_ms = (fim - inicio).num_milliseconds();
    let (status, linhas, erro, produto) = match resultado {
        Ok((produto, n)) => (
            "sucesso".to_owned(),
            i64::try_from(n).unwrap_or(i64::MAX),
            None,
            Some(produto),
        ),
        Err(e) => {
            tracing::error!(erro = %e, modulo, "módulo do pipeline falhou");
            ("erro".to_owned(), 0, Some(e.to_string()), None)
        }
    };
    let execucao = ExecucaoModulo {
        modulo: modulo.to_owned(),
        status,
        linhas,
        duracao_ms,
        erro,
        inicio,
        fim,
    };
    if let Err(e) = derivadas::registrar_execucao(pool, data_ref, &execucao).await {
        tracing::error!(erro = %e, "falha ao registrar a execução do pipeline");
    }
    (execucao, produto)
}

async fn modulo_classificacao(
    pool: &PgPool,
    base: &[BaseProduto],
    config: &Config,
    data_ref: NaiveDate,
) -> Result<(Vec<ResultadoClassificacao>, u64), ErroEngine> {
    let params = mapeamento::parametros_classificacao(config);
    let produtos: Vec<ProdutoParaClassificar> = base
        .iter()
        .map(|b| ProdutoParaClassificar {
            codigo_estoque: CodigoEstoque::novo(&b.codigo_estoque),
            fora_de_linha: b.fora_de_linha,
            primeira_venda: b.primeira_venda,
            ultima_venda: b.ultima_venda,
            volume_janela_abc: b.volume_540,
        })
        .collect();
    let resultados = classificar(&produtos, data_ref, &params);
    let linhas: Vec<LinhaClassificacao> = resultados
        .iter()
        .map(|r| LinhaClassificacao {
            codigo: r.codigo_estoque.como_str().to_owned(),
            classe: r.classe.como_char().to_string(),
            volume_janela: r.volume_janela,
            percentual_acumulado: r.percentual_acumulado,
            fator_estoque: r.fator_estoque,
        })
        .collect();
    let n = derivadas::salvar_classificacao(pool, data_ref, &linhas).await?;
    Ok((resultados, n))
}

async fn modulo_parametros(
    pool: &PgPool,
    base: &[BaseProduto],
    classes: &HashMap<String, ClasseAbc>,
    config: &Config,
    fator_mes: f64,
    data_ref: NaiveDate,
) -> Result<(HashMap<String, ParametrosEstoque>, u64), ErroEngine> {
    let cfg = mapeamento::parametros_estoque(config);
    let diarias =
        agregacoes::vendas_diarias(pool, data_ref, data_ref - Duration::days(365)).await?;
    let mut por_codigo: HashMap<String, Vec<i64>> = HashMap::new();
    for (codigo, qtd) in diarias {
        por_codigo.entry(codigo).or_default().push(qtd);
    }

    let mut params = HashMap::with_capacity(base.len());
    let mut linhas = Vec::with_capacity(base.len());
    for b in base {
        let classe = classes
            .get(&b.codigo_estoque)
            .copied()
            .unwrap_or(ClasseAbc::C);
        let meta = mapeamento::meta_dias(config, classe);
        let vendas = por_codigo
            .get(&b.codigo_estoque)
            .map_or(&[][..], Vec::as_slice);
        let p = calcular_parametros(vendas, meta, fator_mes, &cfg);
        linhas.push(LinhaParametro {
            codigo: b.codigo_estoque.clone(),
            media_diaria: p.media_diaria,
            desvio: p.desvio,
            coef_variacao: p.coef_variacao,
            dias_com_vendas: p.dias_com_vendas,
            outliers_detectados: p.outliers_detectados,
            estoque_minimo: p.estoque_minimo,
            estoque_seguranca: p.estoque_seguranca,
            estoque_total_recomendado: p.estoque_total_recomendado,
            sem_historico_confiavel: p.status == StatusParametros::SemHistoricoConfiavel,
            fator_sazonal: fator_mes,
        });
        params.insert(b.codigo_estoque.clone(), p);
    }
    let n = derivadas::salvar_parametros(pool, data_ref, &linhas).await?;
    Ok((params, n))
}

async fn modulo_alertas(
    pool: &PgPool,
    base: &[BaseProduto],
    classes: &HashMap<String, ClasseAbc>,
    params: &HashMap<String, ParametrosEstoque>,
    config: &Config,
    data_ref: NaiveDate,
) -> Result<((), u64), ErroEngine> {
    let parametros = mapeamento::parametros_alerta(config);
    let entradas: Vec<EntradaAlerta> = base
        .iter()
        .filter_map(|b| {
            let classe = *classes.get(&b.codigo_estoque)?;
            let p = params.get(&b.codigo_estoque)?;
            let cobertura = if p.media_diaria > 0.0 {
                f64::from(b.qtd_disponivel) / p.media_diaria
            } else {
                999.0
            };
            Some(EntradaAlerta {
                codigo_estoque: CodigoEstoque::novo(&b.codigo_estoque),
                classe,
                fora_de_linha: b.fora_de_linha,
                media_diaria: p.media_diaria,
                qtd_disponivel: i64::from(b.qtd_disponivel),
                estoque_total_recomendado: p.estoque_total_recomendado,
                cobertura_dias: cobertura,
            })
        })
        .collect();
    let alertas = gerar_alertas(&entradas, &parametros);
    let linhas: Vec<LinhaAlerta> = alertas
        .iter()
        .map(|a| LinhaAlerta {
            codigo: a.codigo_estoque.como_str().to_owned(),
            prioridade: prioridade_str(a.prioridade).to_owned(),
            classe: a.classe.como_char().to_string(),
            qtd_sugerida: a.qtd_sugerida,
            cobertura_dias: a.cobertura_dias,
        })
        .collect();
    let n = derivadas::salvar_alertas(pool, data_ref, &linhas).await?;
    Ok(((), n))
}

async fn modulo_fora_de_linha(
    pool: &PgPool,
    base: &[BaseProduto],
    classes: &HashMap<String, ClasseAbc>,
    config: &Config,
    data_ref: NaiveDate,
) -> Result<((), u64), ErroEngine> {
    let parametros = mapeamento::parametros_ciclo_vida(config);
    let sugestoes: Vec<LinhaSugestao> = base
        .iter()
        .filter_map(|b| {
            let classe = *classes.get(&b.codigo_estoque)?;
            let dias_sem_venda = b.ultima_venda.map(|u| (data_ref - u).num_days());
            let entrada = EntradaCicloVida {
                codigo_estoque: CodigoEstoque::novo(&b.codigo_estoque),
                fora_de_linha: b.fora_de_linha,
                classe,
                vendas_12m: b.vendas_365,
                volume_12m: b.volume_540,
                dias_sem_venda,
            };
            let s = analisar(&entrada, &parametros)?;
            Some(LinhaSugestao {
                codigo: s.codigo_estoque.como_str().to_owned(),
                acao: acao_str(s.acao).to_owned(),
                pontuacao: i16::try_from(s.pontuacao).unwrap_or(0),
                nivel_certeza: certeza_str(s.nivel_certeza).to_owned(),
                criterios: s
                    .criterios
                    .iter()
                    .map(|c| c.como_str().to_owned())
                    .collect(),
            })
        })
        .collect();
    let n = derivadas::salvar_sugestoes(pool, data_ref, &sugestoes).await?;
    Ok(((), n))
}

async fn carregar_fatores(pool: &PgPool) -> Result<FatoresSazonais, ErroEngine> {
    let pares = db_sazon::listar(pool).await?;
    let mut fatores = [1.0_f64; 12];
    for (mes, fator) in pares {
        if (1..=12).contains(&mes) {
            fatores[usize::try_from(mes - 1).unwrap_or(0)] = fator;
        }
    }
    Ok(FatoresSazonais::novo(fatores))
}

fn prioridade_str(prioridade: Prioridade) -> &'static str {
    match prioridade {
        Prioridade::Critico => "critico",
        Prioridade::Alto => "alto",
        Prioridade::Medio => "medio",
    }
}

fn acao_str(acao: AcaoSugerida) -> &'static str {
    match acao {
        AcaoSugerida::Sair => "sair",
        AcaoSugerida::Voltar => "voltar",
    }
}

fn certeza_str(nivel: NivelCerteza) -> &'static str {
    match nivel {
        NivelCerteza::Alta => "alta",
        NivelCerteza::Media => "media",
        NivelCerteza::Baixa => "baixa",
    }
}
