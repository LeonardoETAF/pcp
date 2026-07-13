-- Vendas diárias por produto/variação (contrato doc 05 §2.1 / doc 04 §2.1).
-- Granularidade: PODE haver várias linhas por (dt_ref, codigo_estoque) — variações
-- LISO/PERSONALIZADO; a consolidação é feita na leitura (doc 02 §1), não aqui.
-- Idempotência por dia (CLAUDE.md §6): reprocessar uma data = DELETE WHERE dt_ref = $1 + INSERT.
CREATE TABLE pcp.vendas_dia (
    id               bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    dt_ref           date NOT NULL,
    codigo_estoque   text NOT NULL,
    sku              text,
    produto          text,
    configuracao     text,
    qtd_vendida      integer NOT NULL,
    is_personalizado boolean NOT NULL,
    -- Coluna de controle p/ auditoria e retenção (CLAUDE.md §9).
    ingerido_em      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT vendas_dia_codigo_nao_vazio CHECK (length(btrim(codigo_estoque)) > 0),
    CONSTRAINT vendas_dia_qtd_nao_negativa CHECK (qtd_vendida >= 0)
);

-- Histórico por produto (CLAUDE.md §6/§15): consultas "últimas vendas do produto".
CREATE INDEX vendas_dia_codigo_dt_idx ON pcp.vendas_dia (codigo_estoque, dt_ref DESC);
-- Por dia: reprocesso e pré-validação do pipeline (contagem de vendas do dia, doc 05 §3).
CREATE INDEX vendas_dia_dt_idx ON pcp.vendas_dia (dt_ref);
