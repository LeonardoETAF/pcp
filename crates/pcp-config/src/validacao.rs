//! Validação de invariantes de negócio que o esquema YAML não garante sozinho
//! (doc 02 §11 — ex.: `pareto_a < pareto_b`, clamps coerentes, metas que somam 100).

use crate::erro::ErroConfig;
use crate::modelo::{
    Alertas, Classificacao, Config, ForaDeLinha, ParametrosEstoque, Reposicao, Sazonalidade,
};

/// Valida invariantes de negócio da configuração.
///
/// # Errors
/// [`ErroConfig::Validacao`] com a lista de todas as invariantes violadas, se houver.
pub fn validar(c: &Config) -> Result<(), ErroConfig> {
    let mut erros = Vec::new();
    validar_classificacao(&c.classificacao, &mut erros);
    validar_parametros(&c.parametros_estoque, &mut erros);
    validar_sazonalidade(&c.sazonalidade, &mut erros);
    validar_alertas(&c.alertas, &mut erros);
    validar_reposicao(&c.reposicao, &mut erros);
    validar_fora_de_linha(&c.fora_de_linha, &mut erros);
    validar_metas(c, &mut erros);
    if erros.is_empty() {
        Ok(())
    } else {
        Err(ErroConfig::Validacao(erros))
    }
}

fn validar_classificacao(c: &Classificacao, erros: &mut Vec<String>) {
    if c.pareto_a == 0 || c.pareto_a >= c.pareto_b {
        erros.push(format!(
            "classificacao: pareto_a ({}) deve estar em 1..pareto_b ({})",
            c.pareto_a, c.pareto_b
        ));
    }
    if c.pareto_b > 100 {
        erros.push(format!(
            "classificacao: pareto_b ({}) deve ser <= 100",
            c.pareto_b
        ));
    }
    for (nome, dias) in [
        ("janela_abc_dias", c.janela_abc_dias),
        ("janela_classe_d_dias", c.janela_classe_d_dias),
        ("janela_produto_novo_dias", c.janela_produto_novo_dias),
    ] {
        if dias == 0 {
            erros.push(format!("classificacao: {nome} deve ser > 0"));
        }
    }
    let fe = &c.fator_estoque;
    for (classe, fator) in [
        ("A", fe.a),
        ("B", fe.b),
        ("C", fe.c),
        ("D", fe.d),
        ("F", fe.f),
        ("N", fe.n),
    ] {
        if fator <= 0.0 {
            erros.push(format!(
                "classificacao.fator_estoque[{classe}] ({fator}) deve ser > 0"
            ));
        }
    }
}

fn validar_parametros(p: &ParametrosEstoque, erros: &mut Vec<String>) {
    if p.janela_vendas_meses == 0 {
        erros.push("parametros_estoque: janela_vendas_meses deve ser > 0".into());
    }
    if p.min_dias_com_vendas == 0 {
        erros.push("parametros_estoque: min_dias_com_vendas deve ser > 0".into());
    }
    if p.outlier_iqr_mult <= 0.0 {
        erros.push("parametros_estoque: outlier_iqr_mult deve ser > 0".into());
    }
    if p.z_score_seguranca <= 0.0 {
        erros.push("parametros_estoque: z_score_seguranca deve ser > 0".into());
    }
    if p.dias_base_minimo == 0 {
        erros.push("parametros_estoque: dias_base_minimo deve ser > 0".into());
    }
    if p.teto_cobertura_dias < p.dias_base_minimo {
        erros.push(format!(
            "parametros_estoque: teto_cobertura_dias ({}) deve ser >= dias_base_minimo ({})",
            p.teto_cobertura_dias, p.dias_base_minimo
        ));
    }
    if !(p.fracao_minimo > 0.0 && p.fracao_minimo <= 1.0) {
        erros.push(format!(
            "parametros_estoque: fracao_minimo ({}) deve estar em (0, 1]",
            p.fracao_minimo
        ));
    }
    // Zero é válido e significativo: "produto sem histórico não recomenda nada" (decisão do dono,
    // 2026-07-10). O que não se admite é média negativa nem um teto abaixo do piso.
    let d = &p.defaults_sem_historico;
    if d.media < 0.0 || !d.media.is_finite() {
        erros.push("parametros_estoque.defaults_sem_historico: media deve ser >= 0".into());
    }
    if d.recomendado_max < d.min {
        erros.push(format!(
            "defaults_sem_historico: recomendado_max ({}) deve ser >= min ({})",
            d.recomendado_max, d.min
        ));
    }
}

fn validar_sazonalidade(s: &Sazonalidade, erros: &mut Vec<String>) {
    if s.clamp_min <= 0.0 || s.clamp_min >= s.clamp_max {
        erros.push(format!(
            "sazonalidade: deve valer 0 < clamp_min ({}) < clamp_max ({})",
            s.clamp_min, s.clamp_max
        ));
    }
    if s.atualizar_apos_dias == 0 {
        erros.push("sazonalidade: atualizar_apos_dias deve ser > 0".into());
    }
}

fn validar_alertas(a: &Alertas, erros: &mut Vec<String>) {
    if !(a.critico_pct > 0.0
        && a.critico_pct < a.alto_pct
        && a.alto_pct < a.medio_pct
        && a.medio_pct <= 1.0)
    {
        erros.push(format!(
            "alertas: deve valer 0 < critico_pct ({}) < alto_pct ({}) < medio_pct ({}) <= 1",
            a.critico_pct, a.alto_pct, a.medio_pct
        ));
    }
}

fn validar_reposicao(r: &Reposicao, erros: &mut Vec<String>) {
    let u = &r.fator_urgencia;
    if !(u.cobertura_lt_7 >= u.cobertura_lt_15 && u.cobertura_lt_15 >= u.default && u.default > 0.0)
    {
        erros.push(
            "reposicao.fator_urgencia: deve valer cobertura_lt_7 >= cobertura_lt_15 >= default > 0"
                .into(),
        );
    }
    if r.protecao_ruptura_dias == 0 {
        erros.push("reposicao: protecao_ruptura_dias deve ser > 0".into());
    }
    if r.aprovacao_automatica.qtd_max == 0 {
        erros.push("reposicao.aprovacao_automatica: qtd_max deve ser > 0".into());
    }
    let l = &r.lead_time_dias;
    if !(l.alta > 0 && l.alta <= l.media && l.media <= l.baixa) {
        erros.push("reposicao.lead_time_dias: deve valer 0 < alta <= media <= baixa".into());
    }
}

fn validar_fora_de_linha(f: &ForaDeLinha, erros: &mut Vec<String>) {
    if f.limiar_sugerir_saida <= f.limiar_sugerir_volta {
        erros.push("fora_de_linha: limiar_sugerir_saida deve ser > limiar_sugerir_volta".into());
    }
    if f.alta_certeza < f.media_certeza {
        erros.push("fora_de_linha: alta_certeza deve ser >= media_certeza".into());
    }
    for (nome, v) in [
        ("limiar_sugerir_saida", f.limiar_sugerir_saida),
        ("limiar_sugerir_volta", f.limiar_sugerir_volta),
        ("alta_certeza", f.alta_certeza),
        ("media_certeza", f.media_certeza),
    ] {
        if v > 20 {
            erros.push(format!(
                "fora_de_linha: {nome} ({v}) excede a escala 0..=20 (doc 02 §8)"
            ));
        }
    }
}

fn validar_metas(c: &Config, erros: &mut Vec<String>) {
    let m = &c.metas_estoque_fisico_pct;
    let soma = m.a + m.b + m.c + m.d;
    if soma != 100 {
        erros.push(format!(
            "metas_estoque_fisico_pct: soma A+B+C+D = {soma}, deveria ser 100"
        ));
    }
    let l = &c.limiar_critico_dias;
    if !(l.a >= l.b && l.b >= l.c && l.c > 0) {
        erros.push("limiar_critico_dias: deve valer A >= B >= C > 0".into());
    }
    let mc = &c.metas_cobertura_dias;
    for (classe, dias) in [
        ("A", mc.a),
        ("B", mc.b),
        ("C", mc.c),
        ("D", mc.d),
        ("F", mc.f),
        ("N", mc.n),
        ("default", mc.default),
    ] {
        if dias == 0 {
            erros.push(format!("metas_cobertura_dias[{classe}] deve ser > 0"));
        }
    }
}
