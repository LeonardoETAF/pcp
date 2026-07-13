-- Corrige o GRÃO do domínio: a unidade de planejamento é a LINHA DE ESTOQUE do One
-- (prd.f03005.est_id = item × configuração de cor), não o item (itm_id).
--
-- Evidência (docs/integracao/acesso-direto-one.md): os três produtos de referência que o
-- PRD §11 manda usar na regressão contra o legado — 6797, 10001, 10473 — não existem como
-- ITM_ID; existem como EST_ID, cada um com sua configuração ("COR DA TAMPA: PRETO", etc.).
-- O `codigo_estoque` do legado sempre foi o EST_ID.
--
-- Consequências desta correção:
--   * a coluna `configuracao` (§12) deixa de ser sempre nula: recebe EST_DCONF;
--   * o universo sai de ~1.6 mil itens para ~24 mil linhas de estoque;
--   * a venda passa a vir do KARDEX (prd.f03007), única fonte que amarra a saída à linha de
--     estoque (CDX_ESTQ → EST_ID em 100% das 795.814 linhas). Os itens de pedido (f05001) NÃO
--     servem: seu ITMP_CNF descreve a configuração comercial (cor do canudo, da estampa) e o
--     ITMP_ESTM aponta para a linha do produto LISO reservado — outro item, não o vendido
--     (confirmado pelo suporte do One).
--
-- O schema `bronze` é cru e re-derivável do One (ver 0013), então as duas tabelas são recriadas
-- em vez de migradas. As tabelas de domínio são repovoadas pelo backfill + reprocesso do motor,
-- que é idempotente por data (CLAUDE.md §3.3).

DROP TABLE IF EXISTS bronze.one_estoque;
DROP TABLE IF EXISTS bronze.one_venda;

-- Snapshot de estoque cru. 1 linha por (dia, LINHA DE ESTOQUE).
CREATE TABLE bronze.one_estoque (
    data_ref      date    NOT NULL,
    est_id        bigint  NOT NULL,   -- linha de estoque (EST_ID) = codigo_estoque do domínio
    est_itm       bigint  NOT NULL,   -- item pai (EST_ITM → ITM_ID)
    est_cnf       bigint,             -- id da configuração (EST_CNF)
    est_dconf     text,               -- configuração legível (EST_DCONF), ex.: "COR DO PRODUTO: PRETO"
    itm_sku       text,
    itm_desc      text,
    est_qtde      integer NOT NULL,   -- saldo físico (EST_QTDE)
    est_qtdd      integer NOT NULL,   -- saldo disponível canônico (EST_QTDD)
    est_qtem      integer,            -- estoque mínimo do ERP (referência)
    est_flin      boolean NOT NULL,   -- fora de linha (EST_FLIN)
    itm_proda     boolean NOT NULL,   -- personalizável (ITM_PRODA)
    capturado_em  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (data_ref, est_id)
);

-- Vendas cru, do kardex. 1 linha por (dia, LINHA DE ESTOQUE).
CREATE TABLE bronze.one_venda (
    cdx_datc      date    NOT NULL,   -- data do movimento (CDX_DATC)
    cdx_estq      bigint  NOT NULL,   -- linha de estoque (CDX_ESTQ → EST_ID)
    itm_sku       text,
    itm_desc      text,
    est_dconf     text,
    cdx_qtd       integer NOT NULL,   -- líquido vendido no dia (VENDA menos DEVOLUCAO_VENDA)
    itm_proda     boolean NOT NULL,
    capturado_em  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (cdx_datc, cdx_estq)
);

CREATE INDEX one_estoque_dt_idx ON bronze.one_estoque (data_ref DESC);
CREATE INDEX one_venda_dt_idx   ON bronze.one_venda (cdx_datc DESC);

-- A coluna-base da retenção de vendas mudou de PEDV_DATC (pedido) para CDX_DATC (kardex).
UPDATE pcp.retencao_politica
   SET base_coluna = 'cdx_datc',
       observacao  = 'Cru do One (kardex, re-derivável); 24 meses.'
 WHERE dataset = 'bronze.one_venda';

-- O grão mudou: todo dado derivado do grão antigo (itm_id) é inválido. O backfill do One
-- repovoa vendas/snapshot e o motor reprocessa as derivadas por data.
TRUNCATE pcp.vendas_dia, pcp.estoque_snapshot,
         pcp.classificacao, pcp.estoque_param, pcp.alerta, pcp.sugestao_ciclo_vida
    RESTART IDENTITY CASCADE;
DELETE FROM bronze.sincronizacao WHERE fonte = 'vendas';
