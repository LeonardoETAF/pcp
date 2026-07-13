-- Solicitação de Produção (doc 03 §4.3) — escrita do USUÁRIO, auditada (CLAUDE.md §7.2/§7.5).
-- Substitui o "setTimeout simulado" do legado: persiste de verdade e tem máquina de estados
-- (pcp-core: pendente → aprovada → em_producao → concluida, e pendente → recusada).
CREATE TABLE pcp.solicitacao_producao (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo_estoque  text NOT NULL,
    qtd_solicitada  bigint NOT NULL,
    prioridade      text NOT NULL,
    lead_time_dias  integer NOT NULL,
    prazo           date NOT NULL,
    solicitante_id  uuid NOT NULL REFERENCES pcp.usuario (id),
    justificativa   text,
    estado          text NOT NULL DEFAULT 'pendente',
    criado_em       timestamptz NOT NULL DEFAULT now(),
    atualizado_em   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT solicitacao_qtd_positiva CHECK (qtd_solicitada > 0),
    CONSTRAINT solicitacao_prioridade_valida CHECK (prioridade IN ('alta', 'media', 'baixa')),
    CONSTRAINT solicitacao_estado_valido
        CHECK (estado IN ('pendente', 'aprovada', 'em_producao', 'concluida', 'recusada'))
);
CREATE INDEX solicitacao_codigo_idx ON pcp.solicitacao_producao (codigo_estoque, criado_em DESC);
CREATE INDEX solicitacao_estado_idx ON pcp.solicitacao_producao (estado, criado_em);

-- Trilha de auditoria (§7.5): quem, quando e o VALOR ANTERIOR (de_estado) de cada transição.
-- Criação = evento com de_estado NULL.
CREATE TABLE pcp.solicitacao_evento (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    solicitacao_id  uuid NOT NULL REFERENCES pcp.solicitacao_producao (id) ON DELETE CASCADE,
    de_estado       text,
    para_estado     text NOT NULL,
    por_id          uuid NOT NULL REFERENCES pcp.usuario (id),
    observacao      text,
    em              timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX solicitacao_evento_idx ON pcp.solicitacao_evento (solicitacao_id, em);

-- Retenção (CLAUDE.md §9 / §13: nenhuma tabela sem política). Registro operacional + auditoria:
-- mantidos permanentemente (retencao_dias NULL). Decisão explícita do dono pode ajustar depois.
INSERT INTO pcp.retencao_politica (dataset, retencao_dias, base_coluna, observacao) VALUES
    ('pcp.solicitacao_producao', NULL, 'criado_em', 'Registro operacional de producao.'),
    ('pcp.solicitacao_evento', NULL, 'em', 'Trilha de auditoria das solicitacoes (§7.5).');
