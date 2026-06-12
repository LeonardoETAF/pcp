-- Schema dedicado do módulo PCP (CLAUDE.md §0): TODAS as tabelas vivem em `pcp`,
-- para que o PCP nasça coeso como módulo do futuro ERP e um próximo módulo (ex.: financeiro)
-- entre ao lado sem tocar nestas tabelas.
CREATE SCHEMA IF NOT EXISTS pcp;

-- Política de retenção por dataset (CLAUDE.md §9 / doc 07 §6), prevista desde o dia 1.
-- Um futuro job de expurgo lê esta tabela. `retencao_meses` NULL = permanente.
-- `base_coluna` é a coluna usada para datar a retenção.
CREATE TABLE pcp.retencao_politica (
    dataset        text PRIMARY KEY,
    retencao_meses integer,
    base_coluna    text NOT NULL,
    observacao     text,
    atualizado_em  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT retencao_meses_positiva CHECK (retencao_meses IS NULL OR retencao_meses > 0)
);

COMMENT ON TABLE pcp.retencao_politica IS
    'Política de retenção por dataset (CLAUDE.md §9). retencao_meses NULL = permanente.';

INSERT INTO pcp.retencao_politica (dataset, retencao_meses, base_coluna, observacao) VALUES
    ('pcp.vendas_dia',       NULL, 'dt_ref', 'Permanente: base de todos os cálculos (doc 07 §6).'),
    ('pcp.estoque_snapshot', 24,   'dt_ref', 'Mínimo de 24 meses (doc 07 §6).');

-- Extensibilidade financeira (CLAUDE.md §6): a dimensão de custo/preço por produto está
-- ADIADA, mas não inviabilizada — entrará como uma tabela própria em `pcp` (ex.:
-- pcp.produto_financeiro, chaveada por codigo_estoque) sem alterar as tabelas de entrada.
