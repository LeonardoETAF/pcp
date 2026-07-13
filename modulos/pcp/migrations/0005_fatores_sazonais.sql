-- Fatores sazonais dinâmicos (doc 02 §4): um multiplicador por mês (1-12), recalculado
-- mensalmente pelo motor. Tabela pequena e fixa (12 linhas) — sem política de retenção.
CREATE TABLE pcp.fatores_sazonais (
    mes           smallint PRIMARY KEY,
    fator         double precision NOT NULL,
    atualizado_em timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT fatores_sazonais_mes_valido CHECK (mes BETWEEN 1 AND 12)
);
