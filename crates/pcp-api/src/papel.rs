//! Papéis de usuário e hierarquia de autorização (CLAUDE.md §7.3).

/// Papel do usuário, em ordem crescente de privilégio: analista < gestor < admin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Papel {
    Analista,
    Gestor,
    Admin,
}

impl Papel {
    /// Texto persistido no banco.
    #[must_use]
    pub fn como_str(self) -> &'static str {
        match self {
            Papel::Analista => "analista",
            Papel::Gestor => "gestor",
            Papel::Admin => "admin",
        }
    }

    /// Converte do texto persistido; `None` se desconhecido.
    #[must_use]
    pub fn tentar_de(texto: &str) -> Option<Self> {
        match texto {
            "analista" => Some(Papel::Analista),
            "gestor" => Some(Papel::Gestor),
            "admin" => Some(Papel::Admin),
            _ => None,
        }
    }

    /// `true` se este papel tem privilégio igual ou superior a `minimo`.
    #[must_use]
    pub fn pelo_menos(self, minimo: Papel) -> bool {
        self >= minimo
    }
}

#[cfg(test)]
mod testes {
    use super::Papel;

    #[test]
    fn hierarquia() {
        assert!(Papel::Admin.pelo_menos(Papel::Gestor));
        assert!(Papel::Admin.pelo_menos(Papel::Analista));
        assert!(Papel::Gestor.pelo_menos(Papel::Analista));
        assert!(Papel::Analista.pelo_menos(Papel::Analista));
        assert!(!Papel::Analista.pelo_menos(Papel::Gestor));
        assert!(!Papel::Gestor.pelo_menos(Papel::Admin));
    }

    #[test]
    fn ida_e_volta_texto() {
        for papel in [Papel::Analista, Papel::Gestor, Papel::Admin] {
            assert_eq!(Papel::tentar_de(papel.como_str()), Some(papel));
        }
        assert_eq!(Papel::tentar_de("root"), None);
    }
}
