-- Schema "bronze": cru do ERP One, camada anticorrupção (docs/integracao/acesso-direto-one.md §2).
-- Decopla a leitura do One (nomes Fxxxxx / colunas crípticas) do domínio limpo (pcp.*): o conector
-- LANDa o cru aqui e transforma para vendas_dia/estoque_snapshot. Quando o One sair, o bronze some;
-- o domínio permanece. Nomes do One preservados de propósito (rastreabilidade da origem).
CREATE SCHEMA IF NOT EXISTS bronze;

-- Snapshot de estoque cru (agregado por produto na leitura; nomes do One). 1 linha por (dia, produto).
CREATE TABLE bronze.one_estoque (
    data_ref      date    NOT NULL,
    itm_id        bigint  NOT NULL,
    itm_sku       text,
    itm_desc      text,
    est_qtde      integer NOT NULL,   -- saldo físico (EST_QTDE)
    est_qtdd      integer NOT NULL,   -- saldo disponível canônico (EST_QTDD)
    est_qtem      integer,            -- estoque mínimo do ERP (referência)
    est_flin      boolean NOT NULL,   -- fora de linha (EST_FLIN)
    itm_proda     boolean NOT NULL,   -- personalizável (ITM_PRODA)
    capturado_em  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (data_ref, itm_id)
);

-- Vendas cru (pedidos não cancelados, consolidado dia×produto). 1 linha por (dia, produto).
CREATE TABLE bronze.one_venda (
    pedv_datc     date    NOT NULL,   -- data do pedido (PEDV_DATC)
    itmp_prd      bigint  NOT NULL,   -- produto (ITMP_PRD → ITM_ID)
    itm_sku       text,
    itm_desc      text,
    itmp_qnt      integer NOT NULL,   -- quantidade do item (ITMP_QNT)
    itm_proda     boolean NOT NULL,
    capturado_em  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (pedv_datc, itmp_prd)
);

-- Venda faturada (COMPLEMENTAR — F10901 cabeçalho + F10911 item). Consolidado dia×produto.
CREATE TABLE bronze.one_fatura (
    fat_dtemi     date    NOT NULL,   -- emissão (FAT_DTEMI)
    fti_prod      bigint  NOT NULL,   -- produto (FTI_PROD)
    fti_dprd      text,
    fti_qtde      integer NOT NULL,   -- quantidade faturada (FTI_QTDE)
    capturado_em  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (fat_dtemi, fti_prod)
);

-- Produção em andamento (COMPLEMENTAR — visibilidade; F06002 ItemProducao). 1 linha por item.
CREATE TABLE bronze.one_producao (
    iprd_id       bigint  PRIMARY KEY,  -- id do item de produção
    iprd_prd      bigint  NOT NULL,     -- produto (IPRD_PRD)
    iprd_qnt      integer NOT NULL,     -- quantidade (IPRD_QNT)
    iprd_stat     text,                 -- status (IPRD_STAT)
    capturado_em  timestamptz NOT NULL DEFAULT now()
);

-- Marca-d'água da sincronização incremental por fonte (vendas/faturas/produção).
CREATE TABLE bronze.sincronizacao (
    fonte         text PRIMARY KEY,
    marca_dagua   date,
    atualizado_em timestamptz NOT NULL DEFAULT now()
);

-- Índices por data (consultas de janela e expurgo de retenção).
CREATE INDEX one_estoque_dt_idx ON bronze.one_estoque (data_ref DESC);
CREATE INDEX one_venda_dt_idx   ON bronze.one_venda (pedv_datc DESC);
CREATE INDEX one_fatura_dt_idx  ON bronze.one_fatura (fat_dtemi DESC);

-- Política de retenção (CLAUDE.md §9/§13): bronze é re-derivável do One, então é transitório.
INSERT INTO pcp.retencao_politica (dataset, retencao_dias, base_coluna, observacao) VALUES
    ('bronze.one_estoque',     730,  'data_ref',      'Cru do One (re-derivável); 24 meses.'),
    ('bronze.one_venda',       730,  'pedv_datc',     'Cru do One (re-derivável); 24 meses.'),
    ('bronze.one_fatura',      730,  'fat_dtemi',     'Cru do One (complementar); 24 meses.'),
    ('bronze.one_producao',    90,   'capturado_em',  'Produção em andamento (complementar); estado recente.'),
    ('bronze.sincronizacao',   NULL, 'atualizado_em', 'Marca-d''água da sincronização; permanente.');
