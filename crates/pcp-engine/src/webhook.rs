//! Notificação de falha do pipeline por webhook (doc 05 §3 — "falha de módulo → notificação").
//! A URL vem da env `PCP_WEBHOOK_FALHA_URL` (operacional e possivelmente com token → fora do
//! código/YAML, §7.4); sem ela, é no-op. Best-effort com backoff: nunca propaga erro (não
//! invalida o pipeline já concluído). Usado quando o pipeline termina em estado parcial.

use chrono::NaiveDate;

use pcp_db::derivadas::ExecucaoModulo;

/// Variável de ambiente com a URL do webhook de falha (vazia/ausente desliga a notificação).
const ENV_URL: &str = "PCP_WEBHOOK_FALHA_URL";
/// Tentativas de envio (com backoff linear entre elas).
const TENTATIVAS: u32 = 3;

/// Notifica a falha do pipeline de `data_ref` por webhook, listando os módulos com erro.
/// No-op se a env não estiver definida ou não houver módulo com erro.
pub async fn notificar_falha(data_ref: NaiveDate, execucoes: &[ExecucaoModulo]) {
    let url = match std::env::var(ENV_URL) {
        Ok(u) if !u.trim().is_empty() => u,
        _ => return,
    };
    let falhas: Vec<_> = execucoes
        .iter()
        .filter(|e| e.status == "erro")
        .map(|e| serde_json::json!({ "modulo": e.modulo, "erro": e.erro }))
        .collect();
    if falhas.is_empty() {
        return;
    }
    let payload = serde_json::json!({
        "evento": "pipeline_falha",
        "data_ref": data_ref.to_string(),
        "modulos_com_erro": falhas,
    });

    // Timeout para o webhook nunca travar o pipeline (best-effort).
    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    for tentativa in 1..=TENTATIVAS {
        match cliente.post(&url).json(&payload).send().await {
            Ok(r) if r.status().is_success() => return,
            Ok(r) => {
                tracing::warn!(status = %r.status(), tentativa, "webhook de falha rejeitado");
            }
            Err(e) => tracing::warn!(erro = %e, tentativa, "webhook de falha não enviado"),
        }
        if tentativa < TENTATIVAS {
            let espera = std::time::Duration::from_millis(500 * u64::from(tentativa));
            tokio::time::sleep(espera).await;
        }
    }
    tracing::error!(%data_ref, "webhook de falha não entregue após {TENTATIVAS} tentativas");
}

#[cfg(test)]
mod testes {
    use super::*;
    use chrono::{DateTime, Utc};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    fn ts() -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(2026, 6, 17)
            .unwrap()
            .and_hms_opt(3, 0, 0)
            .unwrap()
            .and_utc()
    }

    fn modulo(status: &str) -> ExecucaoModulo {
        ExecucaoModulo {
            modulo: "alertas".to_owned(),
            status: status.to_owned(),
            linhas: 0,
            duracao_ms: 1,
            erro: Some("falha simulada".to_owned()),
            inicio: ts(),
            fim: ts(),
        }
    }

    #[tokio::test]
    async fn webhook_dispara_no_erro_e_e_no_op_sem_env() {
        // No-op sem env: não panica nem bloqueia.
        std::env::remove_var(ENV_URL);
        notificar_falha(
            NaiveDate::from_ymd_opt(2026, 6, 17).unwrap(),
            &[modulo("erro")],
        )
        .await;

        // Com env apontando para um listener local: faz POST com o payload esperado.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let porta = listener.local_addr().unwrap().port();
        // SAFETY: teste serial sobre esta env; nenhum outro teste a usa.
        std::env::set_var(ENV_URL, format!("http://127.0.0.1:{porta}/hook"));

        let servidor = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut req = Vec::new();
            let mut buf = [0u8; 1024];
            // Lê até aparecer o corpo (marcador) ou o timeout encerrar.
            while let Ok(n) = stream.read(&mut buf) {
                if n == 0 {
                    break;
                }
                req.extend_from_slice(&buf[..n]);
                if String::from_utf8_lossy(&req).contains("pipeline_falha") {
                    break;
                }
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .ok();
            String::from_utf8_lossy(&req).into_owned()
        });

        notificar_falha(
            NaiveDate::from_ymd_opt(2026, 6, 17).unwrap(),
            &[modulo("erro")],
        )
        .await;
        let recebido = servidor.join().unwrap();
        std::env::remove_var(ENV_URL);

        assert!(recebido.starts_with("POST /hook"), "deveria ser POST /hook");
        assert!(recebido.contains("pipeline_falha"));
        assert!(recebido.contains("\"data_ref\":\"2026-06-17\""));
        assert!(recebido.contains("alertas"));
    }
}
