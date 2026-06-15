//! Mapeamento de `pcp-config` (constantes editáveis do doc 02 §11) para os parâmetros tipados
//! do `pcp-core`. É aqui — na borda — que o núcleo recebe os limiares, sem importar config (§2).

use pcp_config::Config;
use pcp_core::ciclo_vida::ParametrosCicloVida;
use pcp_core::classificacao::{FatoresAbc, ParametrosClassificacao};
use pcp_core::parametros::{DefaultsSemHistorico, ParametrosEstoqueConfig};
use pcp_core::sazonalidade::ParametrosSazonalidade;
use pcp_core::{ClasseAbc, LimiarCriticoDias, ParametrosAlerta};

/// Limiares da classificação (doc 02 §2).
#[must_use]
pub fn parametros_classificacao(c: &Config) -> ParametrosClassificacao {
    let fe = c.classificacao.fator_estoque;
    ParametrosClassificacao {
        janela_abc_dias: i64::from(c.classificacao.janela_abc_dias),
        janela_classe_d_dias: i64::from(c.classificacao.janela_classe_d_dias),
        janela_produto_novo_dias: i64::from(c.classificacao.janela_produto_novo_dias),
        pareto_a: f64::from(c.classificacao.pareto_a),
        pareto_b: f64::from(c.classificacao.pareto_b),
        fatores: FatoresAbc {
            a: fe.a,
            b: fe.b,
            c: fe.c,
            d: fe.d,
            f: fe.f,
            n: fe.n,
        },
    }
}

/// Limiares dos parâmetros de estoque (doc 02 §3).
#[must_use]
pub fn parametros_estoque(c: &Config) -> ParametrosEstoqueConfig {
    let d = c.parametros_estoque.defaults_sem_historico;
    ParametrosEstoqueConfig {
        min_dias_com_vendas: i64::from(c.parametros_estoque.min_dias_com_vendas),
        outlier_iqr_mult: c.parametros_estoque.outlier_iqr_mult,
        z_score_seguranca: c.parametros_estoque.z_score_seguranca,
        teto_cobertura_dias: i64::from(c.parametros_estoque.teto_cobertura_dias),
        fracao_minimo: c.parametros_estoque.fracao_minimo,
        defaults_sem_historico: DefaultsSemHistorico {
            media: d.media,
            minimo: i64::from(d.min),
            seguranca: i64::from(d.seguranca),
            recomendado: i64::from(d.recomendado_max),
        },
    }
}

/// Meta de cobertura em dias da classe vigente (doc 02 §3.6).
#[must_use]
pub fn meta_dias(c: &Config, classe: ClasseAbc) -> i64 {
    let m = c.metas_cobertura_dias;
    let dias = match classe {
        ClasseAbc::A => m.a,
        ClasseAbc::B => m.b,
        ClasseAbc::C => m.c,
        ClasseAbc::D => m.d,
        ClasseAbc::F => m.f,
        ClasseAbc::N => m.n,
    };
    i64::from(dias)
}

/// Limiares de criticidade por classe (doc 02 §5.2). Dias pequenos; `try_from` defensivo.
#[must_use]
pub fn limiar_critico(c: &Config) -> LimiarCriticoDias {
    let l = c.limiar_critico_dias;
    LimiarCriticoDias {
        a: i32::try_from(l.a).unwrap_or(i32::MAX),
        b: i32::try_from(l.b).unwrap_or(i32::MAX),
        c: i32::try_from(l.c).unwrap_or(i32::MAX),
    }
}

/// Limiares dos alertas (doc 02 §6).
#[must_use]
pub fn parametros_alerta(c: &Config) -> ParametrosAlerta {
    ParametrosAlerta {
        critico_pct: c.alertas.critico_pct,
        alto_pct: c.alertas.alto_pct,
        medio_pct: c.alertas.medio_pct,
        elevar_classe_a: c.alertas.elevar_classe_a,
    }
}

/// Limiares do ciclo de vida (doc 02 §8).
#[must_use]
pub fn parametros_ciclo_vida(c: &Config) -> ParametrosCicloVida {
    ParametrosCicloVida {
        limiar_sugerir_saida: i64::from(c.fora_de_linha.limiar_sugerir_saida),
        limiar_sugerir_volta: i64::from(c.fora_de_linha.limiar_sugerir_volta),
        alta_certeza: i64::from(c.fora_de_linha.alta_certeza),
        media_certeza: i64::from(c.fora_de_linha.media_certeza),
    }
}

/// Limiares da sazonalidade (doc 02 §4).
#[must_use]
pub fn parametros_sazonalidade(c: &Config) -> ParametrosSazonalidade {
    ParametrosSazonalidade {
        clamp_min: c.sazonalidade.clamp_min,
        clamp_max: c.sazonalidade.clamp_max,
        atualizar_apos_dias: i64::from(c.sazonalidade.atualizar_apos_dias),
    }
}
