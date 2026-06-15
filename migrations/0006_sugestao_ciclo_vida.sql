-- Generaliza a política de retenção de MESES para DIAS: o doc 07 §6 mistura unidades
-- (ex.: snapshots em meses, sugestões de ciclo de vida em 90 dias). Dias é o denominador comum.
ALTER TABLE pcp.retencao_politica RENAME COLUMN retencao_meses TO retencao_dias;
UPDATE pcp.retencao_politica SET retencao_dias = retencao_dias * 30 WHERE retencao_dias IS NOT NULL;
ALTER TABLE pcp.retencao_politica DROP CONSTRAINT retencao_meses_positiva;
ALTER TABLE pcp.retencao_politica
    ADD CONSTRAINT retencao_dias_positiva CHECK (retencao_dias IS NULL OR retencao_dias > 0);
COMMENT ON TABLE pcp.retencao_politica IS
    'Política de retenção por dataset em DIAS (CLAUDE.md §9). retencao_dias NULL = permanente.';

-- Ciclo de vida do produto como UMA entidade com máquina de estados (doc 04 §3.4),
-- substituindo as duas tabelas do legado (analise_fora_linha + sugestoes_fora_linha), que
-- acumularam 33,7 milhões de linhas sem expurgo (doc 08 §2.2).
CREATE TABLE pcp.sugestao_ciclo_vida (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo_estoque text NOT NULL,
    acao_sugerida  text NOT NULL,
    pontuacao      smallint NOT NULL,
    nivel_certeza  text NOT NULL,
    criterios      text[] NOT NULL DEFAULT '{}',
    estado         text NOT NULL DEFAULT 'gerada',
    data_analise   date NOT NULL,
    aplicado_por   text,
    data_aplicacao timestamptz,
    observacoes    text,
    criado_em      timestamptz NOT NULL DEFAULT now(),
    atualizado_em  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT sugestao_acao_valida CHECK (acao_sugerida IN ('sair', 'voltar')),
    CONSTRAINT sugestao_certeza_valida CHECK (nivel_certeza IN ('alta', 'media', 'baixa')),
    CONSTRAINT sugestao_estado_valido
        CHECK (estado IN ('gerada', 'em_analise', 'aplicada', 'recusada', 'expirada')),
    CONSTRAINT sugestao_pontuacao_valida CHECK (pontuacao BETWEEN 0 AND 20)
);

CREATE INDEX sugestao_ciclo_vida_codigo_idx ON pcp.sugestao_ciclo_vida (codigo_estoque);
-- Apoia o expurgo das não aplicadas (estado, antiguidade).
CREATE INDEX sugestao_ciclo_vida_estado_idx ON pcp.sugestao_ciclo_vida (estado, criado_em);

-- Retenção de 90 dias para sugestões NÃO aplicadas (doc 07 §6 / CLAUDE.md §9).
INSERT INTO pcp.retencao_politica (dataset, retencao_dias, base_coluna, observacao) VALUES
    ('pcp.sugestao_ciclo_vida', 90, 'criado_em', 'Somente nao aplicadas (estado <> aplicada).');
