//! Tradução de erro técnico → mensagem de usuário (CLAUDE.md §12: produto em pt-BR).
//!
//! Nunca exiba `ServerFnError::to_string()` na tela: o `Display` do Leptos prefixa o texto com
//! `"error running server function: "` (inglês) e as variantes de rede/serialização carregam
//! detalhe interno — URL, causa do `reqwest`, nome de campo. O usuário recebe uma frase curta;
//! o detalhe vai para o log.

use leptos::prelude::ServerFnError;

/// Mensagem genérica de fallback: nada do que a causa técnica diz interessa a quem está na tela.
const GENERICA: &str = "Não foi possível concluir. Tente novamente.";
const SEM_CONEXAO: &str = "Sem conexão com o servidor.";

/// Frase curta e em pt-BR para o usuário. Só o texto de `ServerError` é nosso (escrito nas
/// funções de servidor em `crate::api`); as demais variantes viram mensagem genérica.
#[must_use]
pub fn mensagem_usuario(e: &ServerFnError) -> String {
    let texto = match e {
        ServerFnError::ServerError(m) => m.trim(),
        // Rede: o cliente não alcançou o servidor. A causa (DNS, recusa, timeout) fica no log.
        ServerFnError::Request(_) | ServerFnError::Response(_) => SEM_CONEXAO,
        _ => GENERICA,
    };
    if texto.is_empty() {
        GENERICA.to_owned()
    } else {
        texto.to_owned()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn erro_do_servidor_passa_a_nossa_mensagem_sem_prefixo_em_ingles() {
        let e = ServerFnError::ServerError("Sessão expirada. Entre novamente.".to_owned());
        // `to_string()` traria "error running server function: ..."
        assert!(e.to_string().contains("error running server function"));
        assert_eq!(mensagem_usuario(&e), "Sessão expirada. Entre novamente.");
    }

    #[test]
    fn erro_de_rede_nao_vaza_detalhe_tecnico() {
        let e: ServerFnError =
            ServerFnError::Request("reqwest::Error { url: http://127.0.0.1:8080 }".to_owned());
        assert_eq!(mensagem_usuario(&e), SEM_CONEXAO);
    }

    #[test]
    fn erro_de_desserializacao_vira_mensagem_generica() {
        let e: ServerFnError = ServerFnError::Deserialization("missing field `classe`".to_owned());
        assert_eq!(mensagem_usuario(&e), GENERICA);
    }

    #[test]
    fn mensagem_vazia_cai_na_generica() {
        let e = ServerFnError::ServerError(String::new());
        assert_eq!(mensagem_usuario(&e), GENERICA);
    }
}
