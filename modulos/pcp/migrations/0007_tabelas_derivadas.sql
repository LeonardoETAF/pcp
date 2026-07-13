-- Tabelas DERIVADAS, escritas só pelo motor (CLAUDE.md §6). Idempotentes por data.
-- Quantidades em bigint (vêm de i64 no pcp-core).

-- Classificação diária (doc 04 §3.1). Nome honesto `volume_janela` (não `volume_12m` — §3.8).
CREATE TABLE pcp.classificacao (
    dt_calculo           date NOT NULL,
    codigo_estoque       text NOT NULL,
    classe               text NOT NULL,
    volume_janela        bigint NOT NULL,
    percentual_acumulado double precision,
    fator_estoque        double precision NOT NULL,
    PRIMARY KEY (dt_calculo, codigo_estoque),
    CONSTRAINT classificacao_classe_valida CHECK (classe IN ('A', 'B', 'C', 'D', 'F', 'N'))
);
CREATE INDEX classificacao_codigo_idx ON pcp.classificacao (codigo_estoque, dt_calculo DESC);

-- Parâmetros de estoque (doc 04 §3.2) — estado atual por produto (upsert por código).
CREATE TABLE pcp.estoque_param (
    codigo_estoque            text PRIMARY KEY,
    media_diaria              double precision NOT NULL,
    desvio                    double precision NOT NULL,
    coef_variacao             double precision NOT NULL,
    dias_com_vendas           bigint NOT NULL,
    outliers_detectados       bigint NOT NULL,
    estoque_minimo            bigint NOT NULL,
    estoque_seguranca         bigint NOT NULL,
    estoque_total_recomendado bigint NOT NULL,
    sem_historico_confiavel   boolean NOT NULL,
    fator_sazonal             double precision NOT NULL,
    dt_calc                   date NOT NULL,
    atualizado_em             timestamptz NOT NULL DEFAULT now()
);

-- Alertas diários (doc 04 §3.3) — prioridade em campo PRÓPRIO (não no `configuracao` — §6.4).
CREATE TABLE pcp.alerta (
    dt_alerta      date NOT NULL,
    codigo_estoque text NOT NULL,
    prioridade     text NOT NULL,
    classe         text NOT NULL,
    qtd_sugerida   bigint NOT NULL,
    cobertura_dias double precision NOT NULL,
    PRIMARY KEY (dt_alerta, codigo_estoque),
    CONSTRAINT alerta_prioridade_valida CHECK (prioridade IN ('critico', 'alto', 'medio')),
    CONSTRAINT alerta_qtd_nao_negativa CHECK (qtd_sugerida >= 0)
);
CREATE INDEX alerta_codigo_idx ON pcp.alerta (codigo_estoque, dt_alerta DESC);

-- Execuções do pipeline por módulo (doc 05 §3): observabilidade na UI admin.
CREATE TABLE pcp.execucao_pipeline (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    data_ref        date NOT NULL,
    modulo          text NOT NULL,
    status          text NOT NULL,
    linhas_afetadas bigint NOT NULL DEFAULT 0,
    duracao_ms      bigint NOT NULL,
    erro            text,
    inicio          timestamptz NOT NULL,
    fim             timestamptz NOT NULL,
    CONSTRAINT execucao_status_valido CHECK (status IN ('sucesso', 'erro'))
);
CREATE INDEX execucao_data_idx ON pcp.execucao_pipeline (data_ref, modulo);

-- Impede sugestões de ciclo de vida ABERTAS duplicadas por produto (idempotência — doc 04 §3.4).
CREATE UNIQUE INDEX sugestao_aberta_unica
    ON pcp.sugestao_ciclo_vida (codigo_estoque)
    WHERE estado IN ('gerada', 'em_analise');
