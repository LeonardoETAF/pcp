//! Testes do carregamento e validação da configuração de negócio (Prompt 0.2).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use std::path::PathBuf;

use pcp_config::{carregar_de_arquivo, carregar_de_str, validar, Config, Prioridade};

/// Caminho do `config/pcp.config.yaml` de referência, relativo a este crate.
fn caminho_config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/pcp.config.yaml")
}

/// A config de referência sempre carrega e valida — base dos casos negativos.
fn config_valida() -> Config {
    carregar_de_arquivo(caminho_config()).expect("config de referência deve carregar e validar")
}

#[test]
fn carrega_config_de_referencia_com_valores_esperados() {
    let c = config_valida();
    assert_eq!(c.classificacao.janela_abc_dias, 540);
    assert_eq!(c.classificacao.pareto_a, 80);
    assert_eq!(c.classificacao.pareto_b, 95);
    assert!((c.classificacao.fator_estoque.a - 1.20).abs() < f64::EPSILON);
    assert!((c.classificacao.fator_estoque.f - 0.10).abs() < f64::EPSILON);
    assert_eq!(c.metas_cobertura_dias.a, 45);
    assert_eq!(c.metas_cobertura_dias.default, 15);
    assert_eq!(c.limiar_critico_dias.c, 5);
    assert_eq!(c.parametros_estoque.min_dias_com_vendas, 10);
    assert!((c.parametros_estoque.z_score_seguranca - 1.28).abs() < f64::EPSILON);
    assert_eq!(c.parametros_estoque.teto_cobertura_dias, 60);
    assert!((c.sazonalidade.clamp_max - 2.0).abs() < f64::EPSILON);
    assert!(c.alertas.elevar_classe_a);
    assert_eq!(
        c.reposicao.aprovacao_automatica.exceto_prioridade,
        Prioridade::Alta
    );
    assert_eq!(c.reposicao.lead_time_dias.alta, 7);
    assert_eq!(c.metas_estoque_fisico_pct.a, 50);
}

#[test]
fn rejeita_pareto_a_maior_ou_igual_a_b() {
    let mut c = config_valida();
    c.classificacao.pareto_a = c.classificacao.pareto_b;
    assert!(validar(&c).is_err());
}

#[test]
fn rejeita_clamp_sazonal_incoerente() {
    let mut c = config_valida();
    c.sazonalidade.clamp_min = c.sazonalidade.clamp_max + 0.1;
    assert!(validar(&c).is_err());
}

#[test]
fn rejeita_ordem_de_alertas_incoerente() {
    let mut c = config_valida();
    c.alertas.critico_pct = 0.9; // maior que alto_pct/medio_pct
    assert!(validar(&c).is_err());
}

#[test]
fn rejeita_metas_fisicas_que_nao_somam_100() {
    let mut c = config_valida();
    c.metas_estoque_fisico_pct.d = 10; // 50 + 30 + 20 + 10 = 110
    assert!(validar(&c).is_err());
}

#[test]
fn rejeita_fator_estoque_nao_positivo() {
    let mut c = config_valida();
    c.classificacao.fator_estoque.a = 0.0;
    assert!(validar(&c).is_err());
}

#[test]
fn rejeita_lead_time_fora_de_ordem() {
    let mut c = config_valida();
    c.reposicao.lead_time_dias.alta = 99; // alta > media
    assert!(validar(&c).is_err());
}

#[test]
fn rejeita_yaml_com_campo_desconhecido() {
    let yaml = "classificacao:\n  campo_inexistente: 1\n";
    assert!(carregar_de_str(yaml).is_err());
}

#[test]
fn rejeita_yaml_incompleto() {
    let yaml = "classificacao:\n  pareto_a: 80\n";
    assert!(carregar_de_str(yaml).is_err());
}
