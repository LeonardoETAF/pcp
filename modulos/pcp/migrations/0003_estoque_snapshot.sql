-- Snapshot diário COMPLETO de estoque (contrato doc 05 §2.2 / doc 04 §2.2).
-- O snapshot do dia contém TODOS os produtos (não é incremental). Um produto por dia:
-- PK lógica (dt_ref, codigo_estoque). Idempotência por dia (CLAUDE.md §6).
CREATE TABLE pcp.estoque_snapshot (
    dt_ref          date NOT NULL,
    codigo_estoque  text NOT NULL,
    sku             text,
    produto         text,
    configuracao    text,
    qtd_estoque     integer NOT NULL,
    qtd_reserva     integer NOT NULL,
    qtd_disponivel  integer NOT NULL,
    estoque_min_erp integer,
    fora_de_linha   boolean NOT NULL,
    -- Coluna de controle p/ auditoria e retenção (CLAUDE.md §9; snapshot >= 24 meses).
    ingerido_em     timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (dt_ref, codigo_estoque),
    CONSTRAINT estoque_snapshot_codigo_nao_vazio CHECK (length(btrim(codigo_estoque)) > 0),
    -- Invariante do contrato (doc 05 §2.2): disponivel = estoque - reserva.
    CONSTRAINT estoque_snapshot_disponivel_coerente
        CHECK (qtd_disponivel = qtd_estoque - qtd_reserva)
);

-- Histórico por produto (CLAUDE.md §6): evolução do estoque do produto.
CREATE INDEX estoque_snapshot_codigo_dt_idx ON pcp.estoque_snapshot (codigo_estoque, dt_ref DESC);
-- Por dia: localizar o snapshot mais recente (MAX(dt_ref)) e montar gráficos de tendência.
CREATE INDEX estoque_snapshot_dt_idx ON pcp.estoque_snapshot (dt_ref DESC);
