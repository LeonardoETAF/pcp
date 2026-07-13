-- "View" materializada de produtos ativos (doc 04 §5): tabela com TODOS os valores já
-- calculados pelo motor (status, cobertura, recomendada, sugestão — doc 02 §5/§7). A API só
-- LÊ daqui e nunca recalcula regra (CLAUDE.md §3.2). Reescrita a cada pipeline (refresh).
CREATE TABLE pcp.produto_ativo (
    codigo_estoque            text PRIMARY KEY,
    sku                       text,
    produto                   text,
    configuracao              text,
    classe                    text NOT NULL,
    fator_estoque             double precision NOT NULL,
    qtd_estoque               bigint NOT NULL,
    qtd_reserva               bigint NOT NULL,
    qtd_disponivel            bigint NOT NULL,
    media_diaria              double precision NOT NULL,
    coef_variacao             double precision NOT NULL,
    dias_com_vendas           bigint NOT NULL,
    estoque_minimo            bigint NOT NULL,
    estoque_seguranca         bigint NOT NULL,
    estoque_total_recomendado bigint NOT NULL,
    cobertura_dias            double precision NOT NULL,
    status                    text NOT NULL,
    qtd_sugerida              bigint NOT NULL,
    fora_de_linha             boolean NOT NULL,
    volume_janela             bigint NOT NULL,
    dt_ref                    date NOT NULL,
    atualizado_em             timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT produto_ativo_classe_valida CHECK (classe IN ('A', 'B', 'C', 'D', 'F', 'N'))
);

CREATE INDEX produto_ativo_classe_idx ON pcp.produto_ativo (classe);
CREATE INDEX produto_ativo_status_idx ON pcp.produto_ativo (status);
