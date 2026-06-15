//! Limiares da classificação (doc 02 §2). Os valores nascem em `config/pcp.config.yaml`
//! (lidos por pcp-config) e são passados pelo chamador — o núcleo não os conhece (§2/§13).

use crate::tipos::ClasseAbc;

/// Fatores de estoque por classe (doc 02 §2.5).
#[derive(Debug, Clone, Copy)]
pub struct FatoresAbc {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub f: f64,
    pub n: f64,
}

impl FatoresAbc {
    /// Fator de estoque da classe informada.
    #[must_use]
    pub fn de(self, classe: ClasseAbc) -> f64 {
        match classe {
            ClasseAbc::A => self.a,
            ClasseAbc::B => self.b,
            ClasseAbc::C => self.c,
            ClasseAbc::D => self.d,
            ClasseAbc::F => self.f,
            ClasseAbc::N => self.n,
        }
    }
}

/// Limiares completos da classificação ABC+F+D+N (doc 02 §2).
#[derive(Debug, Clone, Copy)]
pub struct ParametrosClassificacao {
    /// Janela da curva ABC, em dias (doc 02 §2.4: 540 = 18 meses).
    pub janela_abc_dias: i64,
    /// Dias sem venda para virar classe D (doc 02 §2.2: 180).
    pub janela_classe_d_dias: i64,
    /// Idade máxima da primeira venda para ser classe N (doc 02 §2.3: 60).
    pub janela_produto_novo_dias: i64,
    /// Teto de percentual acumulado da classe A (doc 02 §2.4: 80).
    pub pareto_a: f64,
    /// Teto de percentual acumulado da classe B (doc 02 §2.4: 95).
    pub pareto_b: f64,
    /// Fatores de estoque por classe.
    pub fatores: FatoresAbc,
}
