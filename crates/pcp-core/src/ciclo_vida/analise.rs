//! Pontuação, decisão e certeza da análise de fora de linha (doc 02 §8). Funções puras.

use super::estado::EstadoCicloVida;
use crate::tipos::{ClasseAbc, CodigoEstoque};

/// Recência máxima (dias desde a última venda) para sugerir VOLTAR à linha (doc 02 §8.2).
const VOLTAR_MAX_DIAS_SEM_VENDA: i64 = 90;
/// Teto da pontuação (CLAUDE.md §4 / doc 02 §8: escala 0–20).
const PONTUACAO_MAXIMA: i64 = 20;

/// Critério atingido na pontuação (doc 02 §8.1). Gravado junto à sugestão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CriterioCicloVida {
    SemVendas12m,
    Vendas12mAte5,
    Vendas12mAte10,
    SemVolume12m,
    Volume12mAte50,
    Volume12mAte100,
    ClasseC,
    ClasseB,
    SemVenda1Ano,
    SemVenda180Dias,
    SemVenda90Dias,
}

impl CriterioCicloVida {
    /// Rótulo persistido (ex.: `SEM_VENDAS_12M`, `CLASSE_C`, `SEM_VENDA_1_ANO`) — doc 02 §8.1.
    #[must_use]
    pub fn como_str(self) -> &'static str {
        match self {
            CriterioCicloVida::SemVendas12m => "SEM_VENDAS_12M",
            CriterioCicloVida::Vendas12mAte5 => "VENDAS_12M_ATE_5",
            CriterioCicloVida::Vendas12mAte10 => "VENDAS_12M_ATE_10",
            CriterioCicloVida::SemVolume12m => "SEM_VOLUME_12M",
            CriterioCicloVida::Volume12mAte50 => "VOLUME_12M_ATE_50",
            CriterioCicloVida::Volume12mAte100 => "VOLUME_12M_ATE_100",
            CriterioCicloVida::ClasseC => "CLASSE_C",
            CriterioCicloVida::ClasseB => "CLASSE_B",
            CriterioCicloVida::SemVenda1Ano => "SEM_VENDA_1_ANO",
            CriterioCicloVida::SemVenda180Dias => "SEM_VENDA_180_DIAS",
            CriterioCicloVida::SemVenda90Dias => "SEM_VENDA_90_DIAS",
        }
    }
}

/// Nível de certeza da sugestão (doc 02 §8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelCerteza {
    Alta,
    Media,
    Baixa,
}

/// Ação sugerida pela análise (doc 02 §8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcaoSugerida {
    Sair,
    Voltar,
}

/// Limiares do ciclo de vida (doc 02 §8 / §11). Originados de pcp-config (§2/§13).
#[derive(Debug, Clone, Copy)]
pub struct ParametrosCicloVida {
    pub limiar_sugerir_saida: i64,
    pub limiar_sugerir_volta: i64,
    pub alta_certeza: i64,
    pub media_certeza: i64,
}

/// Agregados de um produto para a análise (pré-calculados — §15).
#[derive(Debug, Clone)]
pub struct EntradaCicloVida {
    pub codigo_estoque: CodigoEstoque,
    pub fora_de_linha: bool,
    pub classe: ClasseAbc,
    pub vendas_12m: i64,
    pub volume_12m: i64,
    /// Dias desde a última venda; `None` = nunca vendeu.
    pub dias_sem_venda: Option<i64>,
}

/// Sugestão de ciclo de vida (doc 02 §8 / doc 04 §3.4). Nasce no estado `Gerada`.
#[derive(Debug, Clone, PartialEq)]
pub struct SugestaoCicloVida {
    pub codigo_estoque: CodigoEstoque,
    pub acao: AcaoSugerida,
    pub pontuacao: i64,
    pub nivel_certeza: NivelCerteza,
    pub criterios: Vec<CriterioCicloVida>,
    pub estado: EstadoCicloVida,
}

/// Pontuação de risco (doc 02 §8.1) e critérios atingidos. Resultado limitado a 0–20 (§4).
#[must_use]
pub fn pontuar(entrada: &EntradaCicloVida) -> (i64, Vec<CriterioCicloVida>) {
    use CriterioCicloVida as Crit;
    let mut pontos: i64 = 0;
    let mut criterios = Vec::new();

    // Vendas 12m (doc 02 §8.1).
    if entrada.vendas_12m == 0 {
        pontos += 8;
        criterios.push(Crit::SemVendas12m);
    } else if entrada.vendas_12m <= 5 {
        pontos += 6;
        criterios.push(Crit::Vendas12mAte5);
    } else if entrada.vendas_12m <= 10 {
        pontos += 4;
        criterios.push(Crit::Vendas12mAte10);
    }

    // Volume 12m (doc 02 §8.1).
    if entrada.volume_12m == 0 {
        pontos += 6;
        criterios.push(Crit::SemVolume12m);
    } else if entrada.volume_12m <= 50 {
        pontos += 4;
        criterios.push(Crit::Volume12mAte50);
    } else if entrada.volume_12m <= 100 {
        pontos += 2;
        criterios.push(Crit::Volume12mAte100);
    }

    // Classe (doc 02 §8.1).
    match entrada.classe {
        ClasseAbc::C => {
            pontos += 4;
            criterios.push(Crit::ClasseC);
        }
        ClasseAbc::B => {
            pontos += 2;
            criterios.push(Crit::ClasseB);
        }
        _ => {}
    }

    // Recência (doc 02 §8.1): None = nunca vendeu = ≥ 365 dias.
    match entrada.dias_sem_venda {
        None => {
            pontos += 6;
            criterios.push(Crit::SemVenda1Ano);
        }
        Some(dias) if dias >= 365 => {
            pontos += 6;
            criterios.push(Crit::SemVenda1Ano);
        }
        Some(dias) if dias >= 180 => {
            pontos += 4;
            criterios.push(Crit::SemVenda180Dias);
        }
        Some(dias) if dias >= 90 => {
            pontos += 2;
            criterios.push(Crit::SemVenda90Dias);
        }
        Some(_) => {}
    }

    (pontos.min(PONTUACAO_MAXIMA), criterios)
}

/// Nível de certeza pela pontuação (doc 02 §8.3).
#[must_use]
pub fn nivel_certeza(pontuacao: i64, params: &ParametrosCicloVida) -> NivelCerteza {
    if pontuacao >= params.alta_certeza {
        NivelCerteza::Alta
    } else if pontuacao >= params.media_certeza {
        NivelCerteza::Media
    } else {
        NivelCerteza::Baixa
    }
}

/// Decisão SAIR/VOLTAR (doc 02 §8.2), ou `None` se não há sugestão.
#[must_use]
pub fn decidir(
    entrada: &EntradaCicloVida,
    pontuacao: i64,
    params: &ParametrosCicloVida,
) -> Option<AcaoSugerida> {
    // SAIR: produto ATIVO com pontuação >= limiar.
    if !entrada.fora_de_linha && pontuacao >= params.limiar_sugerir_saida {
        return Some(AcaoSugerida::Sair);
    }
    // VOLTAR: produto FORA DE LINHA, pontuação <= limiar, com venda recente.
    if entrada.fora_de_linha
        && pontuacao <= params.limiar_sugerir_volta
        && entrada.vendas_12m > 0
        && entrada
            .dias_sem_venda
            .is_some_and(|dias| dias <= VOLTAR_MAX_DIAS_SEM_VENDA)
    {
        return Some(AcaoSugerida::Voltar);
    }
    None
}

/// Análise completa (doc 02 §8): pontua, decide e classifica a certeza. `None` se não há
/// sugestão. A sugestão nasce no estado `Gerada`.
#[must_use]
pub fn analisar(
    entrada: &EntradaCicloVida,
    params: &ParametrosCicloVida,
) -> Option<SugestaoCicloVida> {
    let (pontuacao, criterios) = pontuar(entrada);
    let acao = decidir(entrada, pontuacao, params)?;
    Some(SugestaoCicloVida {
        codigo_estoque: entrada.codigo_estoque.clone(),
        acao,
        pontuacao,
        nivel_certeza: nivel_certeza(pontuacao, params),
        criterios,
        estado: EstadoCicloVida::Gerada,
    })
}

#[cfg(test)]
mod testes {
    use super::{
        analisar, decidir, nivel_certeza, pontuar, AcaoSugerida, CriterioCicloVida,
        EntradaCicloVida, NivelCerteza, ParametrosCicloVida,
    };
    use crate::ciclo_vida::estado::EstadoCicloVida;
    use crate::tipos::{ClasseAbc, CodigoEstoque};

    fn params() -> ParametrosCicloVida {
        ParametrosCicloVida {
            limiar_sugerir_saida: 8,
            limiar_sugerir_volta: 4,
            alta_certeza: 15,
            media_certeza: 10,
        }
    }

    fn entrada(
        fora: bool,
        classe: ClasseAbc,
        vendas: i64,
        volume: i64,
        dias_sem_venda: Option<i64>,
    ) -> EntradaCicloVida {
        EntradaCicloVida {
            codigo_estoque: CodigoEstoque::novo("X"),
            fora_de_linha: fora,
            classe,
            vendas_12m: vendas,
            volume_12m: volume,
            dias_sem_venda,
        }
    }

    #[test]
    fn pontuacao_maxima_capa_em_20() {
        // 0 vendas (8) + 0 volume (6) + classe C (4) + nunca vendeu (6) = 24 -> capa 20.
        let (p, criterios) = pontuar(&entrada(false, ClasseAbc::C, 0, 0, None));
        assert_eq!(p, 20);
        assert!(criterios.contains(&CriterioCicloVida::SemVendas12m));
        assert!(criterios.contains(&CriterioCicloVida::SemVolume12m));
        assert!(criterios.contains(&CriterioCicloVida::ClasseC));
        assert!(criterios.contains(&CriterioCicloVida::SemVenda1Ano));
    }

    #[test]
    fn pontuacao_por_faixas() {
        // vendas 8 (>5,<=10 -> 4) + volume 80 (<=100 -> 2) + classe A (0) + 100 dias (>=90 -> 2) = 8.
        let (p, _) = pontuar(&entrada(false, ClasseAbc::A, 8, 80, Some(100)));
        assert_eq!(p, 8);
    }

    #[test]
    fn niveis_de_certeza() {
        assert_eq!(nivel_certeza(16, &params()), NivelCerteza::Alta);
        assert_eq!(nivel_certeza(12, &params()), NivelCerteza::Media);
        assert_eq!(nivel_certeza(5, &params()), NivelCerteza::Baixa);
    }

    #[test]
    fn decide_sair_quando_ativo_e_pontuacao_alta() {
        assert_eq!(
            decidir(&entrada(false, ClasseAbc::C, 0, 0, None), 8, &params()),
            Some(AcaoSugerida::Sair)
        );
        // Pontuação 7 (< 8) não sugere sair.
        assert_eq!(
            decidir(&entrada(false, ClasseAbc::C, 0, 0, None), 7, &params()),
            None
        );
    }

    #[test]
    fn decide_voltar_quando_fora_de_linha_com_venda_recente() {
        // fora de linha, pontuação 4 (<=4), vendas>0, 30 dias sem venda (<=90).
        let e = entrada(true, ClasseAbc::A, 12, 500, Some(30));
        assert_eq!(decidir(&e, 4, &params()), Some(AcaoSugerida::Voltar));
        // Sem venda recente (100 dias > 90) -> não volta.
        let e2 = entrada(true, ClasseAbc::A, 12, 500, Some(100));
        assert_eq!(decidir(&e2, 4, &params()), None);
        // Sem nenhuma venda no período -> não volta.
        let e3 = entrada(true, ClasseAbc::A, 0, 0, Some(30));
        assert_eq!(decidir(&e3, 4, &params()), None);
    }

    #[test]
    fn analisar_gera_sugestao_no_estado_gerada() {
        let s = analisar(&entrada(false, ClasseAbc::C, 0, 0, None), &params()).expect("sugestão");
        assert_eq!(s.acao, AcaoSugerida::Sair);
        assert_eq!(s.estado, EstadoCicloVida::Gerada);
        assert_eq!(s.nivel_certeza, NivelCerteza::Alta); // 20 >= 15
    }

    #[test]
    fn analisar_sem_sugestao_retorna_none() {
        // Ativo, com boa rotação: pontuação baixa, sem sugestão.
        let e = entrada(false, ClasseAbc::A, 1000, 100_000, Some(1));
        assert!(analisar(&e, &params()).is_none());
    }
}
