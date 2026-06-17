-- Preferências de exibição por usuário (doc 03 §8): página inicial e tamanho de página.
CREATE TABLE pcp.preferencia_usuario (
    usuario_id     uuid PRIMARY KEY REFERENCES pcp.usuario (id) ON DELETE CASCADE,
    pagina_inicial text NOT NULL DEFAULT 'dashboard',
    tamanho_pagina integer NOT NULL DEFAULT 50,
    atualizado_em  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT preferencia_tamanho_valido CHECK (tamanho_pagina IN (50, 100, 500, 1000))
);

-- Auditoria de override manual de fator sazonal (doc 02 §4 / §7.5): quem, quando, valor anterior.
CREATE TABLE pcp.fator_sazonal_auditoria (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mes            smallint NOT NULL,
    fator_anterior double precision,
    fator_novo     double precision NOT NULL,
    justificativa  text,
    por_id         uuid NOT NULL REFERENCES pcp.usuario (id),
    em             timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT fator_sazonal_aud_mes_valido CHECK (mes BETWEEN 1 AND 12)
);
CREATE INDEX fator_sazonal_aud_idx ON pcp.fator_sazonal_auditoria (em DESC);

-- Retenção (CLAUDE.md §9/§13).
INSERT INTO pcp.retencao_politica (dataset, retencao_dias, base_coluna, observacao) VALUES
    ('pcp.preferencia_usuario', NULL, 'atualizado_em', 'Preferências de exibição do usuário.'),
    ('pcp.fator_sazonal_auditoria', NULL, 'em', 'Trilha de override de sazonalidade (§7.5).');
