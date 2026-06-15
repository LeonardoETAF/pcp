//! Análise de fora de linha / ciclo de vida do produto (doc 02 §8): pontuação de risco,
//! decisão SAIR/VOLTAR, nível de certeza e a máquina de estados da sugestão. Tudo puro.

mod analise;
mod estado;

pub use analise::{
    analisar, decidir, nivel_certeza, pontuar, AcaoSugerida, CriterioCicloVida, EntradaCicloVida,
    NivelCerteza, ParametrosCicloVida, SugestaoCicloVida,
};
pub use estado::{transicionar, ErroTransicao, EstadoCicloVida};
