//! Tipos de domínio do PCP (newtypes/enums — CLAUDE.md §5).

/// Classe de um produto na classificação ABC+F+D+N (doc 02 §2).
/// A ordem das variantes (A < B < C < D < F < N) ordena a fila de produção (doc 02 §6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClasseAbc {
    A,
    B,
    C,
    D,
    F,
    N,
}

impl ClasseAbc {
    /// Caractere persistido (coluna `char(1)` — doc 04 §3.1).
    #[must_use]
    pub fn como_char(self) -> char {
        match self {
            ClasseAbc::A => 'A',
            ClasseAbc::B => 'B',
            ClasseAbc::C => 'C',
            ClasseAbc::D => 'D',
            ClasseAbc::F => 'F',
            ClasseAbc::N => 'N',
        }
    }
}

/// Código de estoque: chave de negócio do produto no ERP (doc 02, glossário). Texto.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodigoEstoque(String);

impl CodigoEstoque {
    /// Cria a partir de qualquer valor conversível em `String`.
    #[must_use]
    pub fn novo(codigo: impl Into<String>) -> Self {
        Self(codigo.into())
    }

    /// Texto do código.
    #[must_use]
    pub fn como_str(&self) -> &str {
        &self.0
    }
}
