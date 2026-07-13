-- Configuração de negócio persistida (doc 02 §11 / CLAUDE.md §3.7). O YAML é só o DEFAULT
-- inicial; a partir da primeira edição, a config efetiva vive aqui e é recarregada a quente
-- (sem reiniciar). Singleton: uma linha (id = true).
CREATE TABLE pcp.config_efetiva (
    id            boolean PRIMARY KEY DEFAULT true,
    valor         jsonb NOT NULL,
    atualizado_em timestamptz NOT NULL DEFAULT now(),
    atualizado_por uuid REFERENCES pcp.usuario (id),
    CONSTRAINT config_efetiva_singleton CHECK (id)
);

-- Auditoria por constante alterada (§7.5): quem, quando, valor anterior e novo.
CREATE TABLE pcp.config_auditoria (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    chave          text NOT NULL,
    valor_anterior text,
    valor_novo     text,
    por_id         uuid NOT NULL REFERENCES pcp.usuario (id),
    em             timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX config_auditoria_em_idx ON pcp.config_auditoria (em DESC);

-- Retenção (CLAUDE.md §9/§13): config efetiva permanente; auditoria permanente (trilha legal).
INSERT INTO pcp.retencao_politica (dataset, retencao_dias, base_coluna, observacao) VALUES
    ('pcp.config_efetiva', NULL, 'atualizado_em', 'Config vigente (singleton).'),
    ('pcp.config_auditoria', NULL, 'em', 'Trilha de auditoria de configuracao (§7.5).');
