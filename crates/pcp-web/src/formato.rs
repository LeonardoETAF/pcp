//! Helpers de apresentação compartilhados (pt-BR, §12). Sem regra de negócio — só formatação e
//! rótulos de exibição reutilizados pelas telas de estoque e detalhe do produto.

/// Inteiro com separador de milhar à brasileira (§12): `1234567` → `1.234.567`.
#[must_use]
pub fn fmt_milhar(n: i64) -> String {
    let negativo = n < 0;
    let digitos = n.unsigned_abs().to_string();
    let n_dig = digitos.len();
    let mut saida = String::with_capacity(n_dig + n_dig / 3 + 1);
    for (i, ch) in digitos.chars().enumerate() {
        if i != 0 && (n_dig - i).is_multiple_of(3) {
            saida.push('.');
        }
        saida.push(ch);
    }
    if negativo {
        format!("-{saida}")
    } else {
        saida
    }
}

/// Número com 1 casa decimal à brasileira (vírgula, §12): `22.4` → `22,4`.
#[must_use]
pub fn fmt_dec1(v: f64) -> String {
    format!("{v:.1}").replace('.', ",")
}

/// Número compacto pt-BR para KPIs grandes (§12): `10_230_601` → `10,2 mi`; `12_400` → `12,4 mil`;
/// abaixo de mil mostra o inteiro com separador de milhar.
#[must_use]
#[allow(clippy::cast_precision_loss)] // KPIs: magnitude pequena o bastante p/ f64 exato
pub fn fmt_compacto(n: i64) -> String {
    let abs = n.unsigned_abs();
    if abs >= 1_000_000 {
        format!("{} mi", fmt_dec1(n as f64 / 1_000_000.0))
    } else if abs >= 10_000 {
        format!("{} mil", fmt_dec1(n as f64 / 1_000.0))
    } else {
        fmt_milhar(n)
    }
}

/// Cobertura: sentinela 999 vira "Sem histórico" (§12); senão 1 casa decimal pt-BR.
#[must_use]
pub fn fmt_cobertura(c: f64) -> String {
    if c >= 999.0 {
        "Sem histórico".to_owned()
    } else {
        fmt_dec1(c)
    }
}

/// Cor (var CSS do semáforo, §12) do status canônico de estoque — usada nas barras e realces.
/// Espelha o mapeamento das `badge--status-*` (doc 02 §5.2).
#[must_use]
pub fn cor_status(codigo: &str) -> &'static str {
    match codigo {
        "sem_estoque" | "critico" => "var(--semaforo-critico)",
        "estoque_baixo" => "var(--semaforo-alto)",
        "baixo" => "var(--semaforo-medio)",
        "adequado" => "var(--semaforo-ok)",
        "alto" | "excessivo" => "var(--semaforo-info)",
        // fora_de_linha / sem_historico / desconhecido: cinza neutro
        _ => "var(--abc-d)",
    }
}

/// Nome de exibição "{produto} - {cor}" — cor = texto após ':' da configuração (doc 02 §10/§12).
#[must_use]
pub fn nome_exibicao(produto: Option<&str>, configuracao: Option<&str>, codigo: &str) -> String {
    let base = produto
        .filter(|s| !s.is_empty())
        .unwrap_or(codigo)
        .to_owned();
    match configuracao.and_then(|c| c.split(':').nth(1)) {
        Some(cor) if !cor.trim().is_empty() => format!("{base} - {}", cor.trim()),
        _ => base,
    }
}

/// Rótulo pt-BR do status canônico (doc 02 §5.2 / §12).
#[must_use]
pub fn rotulo_status(codigo: &str) -> &'static str {
    match codigo {
        "sem_estoque" => "Sem estoque",
        "fora_de_linha" => "Fora de linha",
        "sem_historico" => "Sem histórico",
        "critico" => "Crítico",
        "estoque_baixo" => "Estoque baixo",
        "baixo" => "Baixo",
        "adequado" => "Adequado",
        "alto" => "Alto",
        "excessivo" => "Excessivo",
        _ => "—",
    }
}
