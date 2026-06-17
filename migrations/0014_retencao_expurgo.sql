-- Completa a política de retenção (CLAUDE.md §9/§13) e habilita um expurgo dirigido pela própria
-- tabela `pcp.retencao_politica` (fonte única das janelas). O expurgo (pcp-db) lê estas linhas e
-- apaga o que excede a janela; nenhuma tabela de dados deve acumular sem política (débito do legado).

-- Condição extra opcional por dataset (ex.: só expurgar sugestões NÃO aplicadas). Fragmento SQL
-- controlado por migration (confiável), aplicado pelo expurgo após o filtro de data.
ALTER TABLE pcp.retencao_politica ADD COLUMN IF NOT EXISTS condicao_extra text;

-- Sugestões de ciclo de vida: 90 dias só para as NÃO aplicadas (§9; as aplicadas são histórico).
UPDATE pcp.retencao_politica
   SET condicao_extra = 'estado <> ''aplicada'''
 WHERE dataset = 'pcp.sugestao_ciclo_vida';

-- Políticas que faltavam (tabelas derivadas/log que o §9 nomeia):
INSERT INTO pcp.retencao_politica (dataset, retencao_dias, base_coluna, observacao) VALUES
    ('pcp.alerta',            365, 'dt_alerta',  'Alertas: 12 meses (doc 07 §6).'),
    ('pcp.classificacao',     730, 'dt_calculo', 'Classificação diária: 24 meses (doc 07 §6).'),
    ('pcp.estoque_param',     730, 'dt_calc',    'Parâmetros diários derivados: 24 meses.'),
    ('pcp.execucao_pipeline', 365, 'data_ref',   'Logs do pipeline: 12 meses (doc 07 §6).')
ON CONFLICT (dataset) DO NOTHING;

-- refresh_token tem retenção por EXPIRAÇÃO (expira_em), não por idade fixa — a CHECK proíbe
-- retencao_dias <= 0, então o expurgo o trata à parte (DELETE WHERE expira_em < now()). Documentado:
COMMENT ON TABLE pcp.refresh_token IS
    'Tokens de refresh; retenção por expiração (expira_em) — expurgados quando vencidos (§7/§9).';
