//! Orquestrador diário do PCP (doc 05 §1.2): pré-validação bloqueante → sazonalidade →
//! classificação → parâmetros → alertas → fora de linha. Idempotente por `data_ref`, com
//! isolamento de falha e telemetria por módulo (doc 05 §3). A regra vive no `pcp-core`.

use std::collections::HashMap;
use std::future::Future;

use chrono::{Datelike, Duration, NaiveDate, Utc};

use pcp_config::Config;
use pcp_core::ciclo_vida::analisar;
use pcp_core::parametros::VendaDiaria;
use pcp_core::sazonalidade::FatoresSazonais;
use pcp_core::{
    calcular_parametros, classificar, cobertura_dias, gerar_alertas, qtd_sugerida, status_estoque,
    AcaoSugerida, ClasseAbc, CodigoEstoque, EntradaAlerta, EntradaCicloVida, EntradaStatus,
    NivelCerteza, ParametrosEstoque, Prioridade, ProdutoParaClassificar, ResultadoClassificacao,
    StatusParametros,
};
use pcp_db::agregacoes::{self, BaseProduto};
use pcp_db::derivadas::{
    self, ExecucaoModulo, LinhaAlerta, LinhaClassificacao, LinhaParametro, LinhaProdutoAtivo,
    LinhaSugestao,
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

/// Pré-validação bloqueante (doc 05 §3): exige snapshot do dia e venda recente. `true` = pode
/// processar.
///
/// "Venda de ontem" seria falso toda segunda-feira (e depois de feriado): sábado e domingo não têm
/// venda, então a checagem literal travaria o pipeline uma vez por semana. A janela de tolerância
/// (`pipeline.tolerancia_vendas_dias`) exige venda em ALGUM dos últimos N dias. Tolerância 1
/// restaura a exigência literal do dia anterior.
///
/// # Errors
/// [`ErroEngine`] em falha de infraestrutura ao contar vendas/snapshot.
async fn pre_validar(
    pool: &PgPool,
    config: &Config,
    data_ref: NaiveDate,
) -> Result<bool, ErroEngine> {
    let tolerancia = i64::from(config.pipeline.tolerancia_vendas_dias.max(1));
    let ontem = data_ref - Duration::days(1);
    let vendas_janela =
        agregacoes::contar_vendas_janela(pool, data_ref - Duration::days(tolerancia), ontem)
            .await?;
    let snapshot_atual = agregacoes::contar_snapshot(pool, data_ref).await?;
    if vendas_janela == 0 || snapshot_atual == 0 {
        tracing::warn!(
            %data_ref, vendas_janela, snapshot_atual, tolerancia,
            "pré-validação falhou; pipeline bloqueado"
        );
        return Ok(false);
    }
    // Passou pela tolerância, mas ontem não teve venda: a equipe precisa saber (doc 05 §3).
    if agregacoes::contar_vendas(pool, ontem).await? == 0 {
        tracing::warn!(
            %data_ref, %ontem, tolerancia,
            "sem venda no dia anterior; seguindo pela tolerância da pré-validação"
        );
    }
    Ok(true)
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
    if !pre_validar(pool, config, data_ref).await? {
        return Ok(ResultadoPipeline {
            data_ref,
            status: StatusPipeline::Bloqueado,
            execucoes: Vec::new(),
        });
    }

    // Sazonalidade (failsafe — doc 02 §4.2) e fator do mês corrente.
    sazonalidade::atualizar_fatores(pool, data_ref, mapeamento::parametros_sazonalidade(config))
        .await?;
    let mes_seguinte = proximo_mes(data_ref.month());
    let (fator_de, demanda_seguinte) = antecipacao(pool, data_ref, mes_seguinte).await?;

    let base = agregacoes::base_produtos(
        pool,
        data_ref,
        data_ref - Duration::days(540),
        data_ref - Duration::days(365),
    )
    .await?;

    let mut execucoes = Vec::with_capacity(5);

    // 1. Classificação → fonte das classes (e fator/volume) para os demais módulos.
    let (exec, classes_res) = executar(
        pool,
        data_ref,
        "classificacao",
        modulo_classificacao(pool, &base, config, data_ref),
    )
    .await;
    execucoes.push(exec);
    let classes: HashMap<String, ResultadoClassificacao> = classes_res
        .map(|rs| {
            rs.into_iter()
                .map(|r| (r.codigo_estoque.como_str().to_owned(), r))
                .collect()
        })
        .unwrap_or_default();

    // 2. Parâmetros estatísticos.
    let (exec, params_res) = executar(
        pool,
        data_ref,
        "parametros",
        modulo_parametros(
            pool,
            &base,
            &classes,
            config,
            &fator_de,
            &demanda_seguinte,
            data_ref,
        ),
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

    // 5. Consolidação: status + cobertura + sugestão por produto na "view" materializada
    //    `produto_ativo` (doc 04 §5). É daqui que a API lê, sem recalcular regra (§3.2).
    let (exec, _) = executar(
        pool,
        data_ref,
        "consolidacao",
        modulo_consolidacao(pool, &base, &classes, &params, config, data_ref),
    )
    .await;
    execucoes.push(exec);

    let status = if execucoes.iter().all(|e| e.status == "sucesso") {
        StatusPipeline::Completo
    } else {
        StatusPipeline::Parcial
    };

    // Sinaliza o fim do processamento para a UI em tempo real (SSE — CLAUDE.md §16). Best-effort:
    // a falha em notificar não invalida o pipeline já concluído.
    if let Err(e) = pcp_db::eventos::notificar_pipeline(pool, data_ref, status_codigo(status)).await
    {
        tracing::warn!(erro = %e, "falha ao notificar o fim do pipeline (SSE)");
    }

    // Falha de módulo → notificação por webhook (doc 05 §3). Best-effort, no-op sem a env.
    if status == StatusPipeline::Parcial {
        crate::webhook::notificar_falha(data_ref, &execucoes).await;
    }

    Ok(ResultadoPipeline {
        data_ref,
        status,
        execucoes,
    })
}

/// Código estável do status para o payload de notificação (SSE — §16).
const fn status_codigo(status: StatusPipeline) -> &'static str {
    match status {
        StatusPipeline::Bloqueado => "bloqueado",
        StatusPipeline::Completo => "completo",
        StatusPipeline::Parcial => "parcial",
    }
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
    classes: &HashMap<String, ResultadoClassificacao>,
    config: &Config,
    fator_de: &impl Fn(&str) -> f64,
    demanda_seguinte: &HashMap<String, f64>,
    data_ref: NaiveDate,
) -> Result<(HashMap<String, ParametrosEstoque>, u64), ErroEngine> {
    let cfg = mapeamento::parametros_estoque(config);
    let diarias =
        agregacoes::vendas_diarias(pool, data_ref, data_ref - Duration::days(365)).await?;
    let mut por_codigo: HashMap<String, Vec<VendaDiaria>> = HashMap::new();
    for (codigo, data, qtd) in diarias {
        por_codigo
            .entry(codigo)
            .or_default()
            .push(VendaDiaria { data, qtd });
    }

    let mut params = HashMap::with_capacity(base.len());
    let mut linhas = Vec::with_capacity(base.len());
    for b in base {
        let classe = classes
            .get(&b.codigo_estoque)
            .map_or(ClasseAbc::C, |r| r.classe);
        let meta = mapeamento::meta_dias(config, classe);
        let vendas = por_codigo
            .get(&b.codigo_estoque)
            .map_or(&[][..], Vec::as_slice);
        let fator_sazonal = fator_de(&b.codigo_estoque);
        let dem_seguinte = demanda_seguinte.get(&b.codigo_estoque).copied();
        let p = calcular_parametros(vendas, data_ref, meta, fator_sazonal, dem_seguinte, &cfg);
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
            fator_sazonal,
            demanda_mes_seguinte: p.demanda_mes_seguinte,
        });
        params.insert(b.codigo_estoque.clone(), p);
    }
    let n = derivadas::salvar_parametros(pool, data_ref, &linhas).await?;
    Ok((params, n))
}

async fn modulo_alertas(
    pool: &PgPool,
    base: &[BaseProduto],
    classes: &HashMap<String, ResultadoClassificacao>,
    params: &HashMap<String, ParametrosEstoque>,
    config: &Config,
    data_ref: NaiveDate,
) -> Result<((), u64), ErroEngine> {
    let parametros = mapeamento::parametros_alerta(config);
    let entradas: Vec<EntradaAlerta> = base
        .iter()
        .filter_map(|b| {
            let classe = classes.get(&b.codigo_estoque)?.classe;
            let p = params.get(&b.codigo_estoque)?;
            let cobertura = cobertura_dias(i64::from(b.qtd_disponivel), p.media_diaria);
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
    classes: &HashMap<String, ResultadoClassificacao>,
    config: &Config,
    data_ref: NaiveDate,
) -> Result<((), u64), ErroEngine> {
    let parametros = mapeamento::parametros_ciclo_vida(config);
    let sugestoes: Vec<LinhaSugestao> = base
        .iter()
        .filter_map(|b| {
            let classe = classes.get(&b.codigo_estoque)?.classe;
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

async fn modulo_consolidacao(
    pool: &PgPool,
    base: &[BaseProduto],
    classes: &HashMap<String, ResultadoClassificacao>,
    params: &HashMap<String, ParametrosEstoque>,
    config: &Config,
    data_ref: NaiveDate,
) -> Result<((), u64), ErroEngine> {
    let limiar = mapeamento::limiar_critico(config);
    let linhas: Vec<LinhaProdutoAtivo> = base
        .iter()
        .filter_map(|b| {
            let r = classes.get(&b.codigo_estoque)?;
            let p = params.get(&b.codigo_estoque)?;
            let qtd_disponivel = i64::from(b.qtd_disponivel);
            let cobertura = cobertura_dias(qtd_disponivel, p.media_diaria);
            let status = status_estoque(
                &EntradaStatus {
                    classe: r.classe,
                    fora_de_linha: b.fora_de_linha,
                    media_diaria: p.media_diaria,
                    cobertura_dias: cobertura,
                    qtd_disponivel,
                    estoque_minimo: p.estoque_minimo,
                    estoque_seguranca: p.estoque_seguranca,
                    estoque_total_recomendado: p.estoque_total_recomendado,
                },
                &limiar,
            );
            let sugerida = qtd_sugerida(
                p.estoque_total_recomendado,
                qtd_disponivel,
                b.fora_de_linha,
                p.media_diaria,
            );
            Some(LinhaProdutoAtivo {
                codigo: b.codigo_estoque.clone(),
                sku: b.sku.clone(),
                produto: b.produto.clone(),
                configuracao: b.configuracao.clone(),
                classe: r.classe.como_char().to_string(),
                fator_estoque: r.fator_estoque,
                qtd_estoque: i64::from(b.qtd_estoque),
                qtd_reserva: i64::from(b.qtd_reserva),
                qtd_disponivel,
                media_diaria: p.media_diaria,
                coef_variacao: p.coef_variacao,
                dias_com_vendas: p.dias_com_vendas,
                estoque_minimo: p.estoque_minimo,
                estoque_seguranca: p.estoque_seguranca,
                estoque_total_recomendado: p.estoque_total_recomendado,
                cobertura_dias: cobertura,
                status: status.codigo().to_owned(),
                qtd_sugerida: sugerida,
                fora_de_linha: b.fora_de_linha,
                volume_janela: r.volume_janela,
            })
        })
        .collect();
    let n = derivadas::salvar_produtos_ativos(pool, data_ref, &linhas).await?;
    Ok(((), n))
}

/// Mês seguinte a `mes` (dezembro → janeiro).
const fn proximo_mes(mes: u32) -> u32 {
    if mes == 12 {
        1
    } else {
        mes + 1
    }
}

/// Antecipação do MÊS SEGUINTE (decisão do dono, 2026-07-13). Devolve:
///
/// 1. o resolvedor do fator sazonal por produto — o fator do MÊS SEGUINTE, não o do mês corrente,
///    porque o que se produz hoje serve o mês que vem. Como as curvas vêm do ano passado, isto É a
///    "análise do mês seguinte do ano passado", por produto. Sem curva própria, cai no global;
/// 2. o que cada produto vendia naquele mês, um ano atrás (média por dia corrido). Produto que
///    ainda NÃO EXISTIA lá fica fora do mapa (vira `None`, não zero — ausência de produto não é
///    ausência de demanda).
async fn antecipacao(
    pool: &PgPool,
    data_ref: NaiveDate,
    mes_seguinte: u32,
) -> Result<(impl Fn(&str) -> f64, HashMap<String, f64>), ErroEngine> {
    let fatores = carregar_fatores(pool).await?;
    let fator_global = fatores.obter_fator(mes_seguinte);
    let curvas = db_sazon::carregar_por_produto(pool).await?;
    let idx = usize::try_from(mes_seguinte).unwrap_or(1).max(1) - 1;
    let fator_de = move |codigo: &str| -> f64 {
        curvas
            .get(codigo)
            .and_then(|f| f.get(idx).copied())
            .unwrap_or(fator_global)
    };

    let (inicio, fim) = janela_mes_ano_passado(data_ref, mes_seguinte)?;
    let demanda: HashMap<String, f64> = agregacoes::media_diaria_no_mes(pool, inicio, fim)
        .await?
        .into_iter()
        .collect();
    Ok((fator_de, demanda))
}

/// Janela `[início, fim)` do MÊS SEGUINTE no ANO PASSADO, relativa a `data_ref`.
fn janela_mes_ano_passado(
    data_ref: NaiveDate,
    mes_seguinte: u32,
) -> Result<(NaiveDate, NaiveDate), ErroEngine> {
    // Se o mês seguinte é janeiro, ele já pertence ao ano que vem — o "ano passado" dele é o ano
    // corrente. Nos demais casos, é o ano anterior.
    let ano = if mes_seguinte == 1 {
        data_ref.year()
    } else {
        data_ref.year() - 1
    };
    let inicio = NaiveDate::from_ymd_opt(ano, mes_seguinte, 1).ok_or(ErroEngine::DataInvalida)?;
    let (ano_fim, mes_fim) = if mes_seguinte == 12 {
        (ano + 1, 1)
    } else {
        (ano, mes_seguinte + 1)
    };
    let fim = NaiveDate::from_ymd_opt(ano_fim, mes_fim, 1).ok_or(ErroEngine::DataInvalida)?;
    Ok((inicio, fim))
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
