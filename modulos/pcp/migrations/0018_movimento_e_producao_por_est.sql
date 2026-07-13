-- Dados de apoio da tela de detalhe do produto (doc 03 §4): histórico de MOVIMENTAÇÃO e de
-- PRODUÇÃO, e status de produção — todos por LINHA DE ESTOQUE (est_id), o grão do domínio.
--
-- Movimentação: o kardex do One (prd.f03007) já é a fonte das vendas; aqui ele é landado inteiro
-- (não agregado) por est_id, para a linha do tempo de entradas/saídas. Incremental por data, como
-- as vendas. Só exibição — não alimenta o motor.
--
-- Produção: as ordens (prd.f06002) são por (item, configuração). A tabela one_producao passa a
-- guardar iprd_cnf e a quantidade produzida (iprd_qntt), para casar com a linha de estoque via
-- (est_itm, est_cnf) e distinguir ordem aberta de finalizada.
--
-- Bronze é re-derivável do One (0013): a tabela de produção é recriada; a de movimento é nova. O
-- backfill repovoa. Nenhuma tabela de domínio muda.

-- Kardex por linha de estoque (1 linha por movimento; cdx_id do One é a chave).
CREATE TABLE bronze.one_movimento (
    cdx_id        bigint  PRIMARY KEY,   -- id do movimento no One
    cdx_estq      bigint  NOT NULL,      -- linha de estoque (CDX_ESTQ → EST_ID = codigo_estoque)
    cdx_datc      date    NOT NULL,      -- data do movimento
    cdx_tpmvm     text    NOT NULL,      -- tipo (VENDA, PRODUCAO, INVENTARIO, ...)
    cdx_qtd       integer NOT NULL,      -- quantidade movimentada (sinal do One: saída negativa)
    cdx_sd        bigint,               -- saldo após o movimento (pode ser grande no One)
    capturado_em  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX one_movimento_estq_idx ON bronze.one_movimento (cdx_estq, cdx_datc DESC);
CREATE INDEX one_movimento_dt_idx   ON bronze.one_movimento (cdx_datc DESC);

-- Ordens de produção com a configuração, para ligar à linha de estoque.
DROP TABLE IF EXISTS bronze.one_producao;
CREATE TABLE bronze.one_producao (
    iprd_id       bigint  PRIMARY KEY,   -- id do item de produção
    iprd_prd      bigint  NOT NULL,      -- item pai (IPRD_PRD → ITM_ID = est_itm)
    iprd_cnf      bigint,               -- configuração (IPRD_CNF → est_cnf)
    iprd_qnt      integer NOT NULL,      -- quantidade planejada
    iprd_qntt     integer NOT NULL,      -- quantidade produzida (IPRD_QNTT)
    iprd_stat     text,                 -- status (AGUARDANDO, PRODUCAO, FINALIZADO, CANCELADO)
    iprd_lote     bigint,               -- lote/ordem (número)
    aud_date      date,                 -- data do registro no One
    capturado_em  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX one_producao_prd_cnf_idx ON bronze.one_producao (iprd_prd, iprd_cnf);

-- Retenção: movimento acompanha as vendas (24m); produção é estado atual (full refresh).
INSERT INTO pcp.retencao_politica (dataset, retencao_dias, base_coluna, observacao) VALUES
    ('bronze.one_movimento', 730, 'cdx_datc', 'Kardex por linha de estoque (re-derivável); 24 meses.')
ON CONFLICT (dataset) DO NOTHING;
UPDATE pcp.retencao_politica
   SET base_coluna = 'capturado_em', retencao_dias = 90,
       observacao = 'Ordens de produção (estado atual, re-derivável); recente.'
 WHERE dataset = 'bronze.one_producao';
