-- Sazonalidade POR PRODUTO (doc 02 §4, estendido — decisão do dono, 2026-07-13).
--
-- Até aqui o fator sazonal era ÚNICO para a empresa inteira: dezembro valia 1.67 para todos os
-- produtos. Isso erra quem tem ciclo próprio (um item que vende forte em junho, enquanto o resto
-- da empresa está fraco, recebia o fator de junho da empresa: 0.83).
--
-- Agora cada produto ganha sua curva de 12 fatores, calculada sobre a PRÓPRIA série. Produto sem
-- histórico que sustente curva própria (poucos meses com venda) NÃO entra aqui — o motor cai no
-- fator global, que continua existindo em pcp.fatores_sazonais. Ver
-- pcp_core::sazonalidade::calcular_fatores_produto.

CREATE TABLE pcp.fator_sazonal_produto (
    codigo_estoque  text        NOT NULL,
    mes             smallint    NOT NULL CHECK (mes BETWEEN 1 AND 12),
    fator           double precision NOT NULL,
    atualizado_em   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (codigo_estoque, mes)
);

-- A leitura é sempre "os 12 fatores deste produto".
CREATE INDEX fator_sazonal_produto_cod_idx ON pcp.fator_sazonal_produto (codigo_estoque);

-- Estado atual (full refresh a cada recálculo), como pcp.fatores_sazonais: sem expurgo por data,
-- mas com política registrada — nenhuma tabela fica fora do controle de retenção (§9/§13).
INSERT INTO pcp.retencao_politica (dataset, retencao_dias, base_coluna, observacao) VALUES
    ('pcp.fator_sazonal_produto', NULL, 'atualizado_em',
     'Estado atual (full refresh no recálculo da sazonalidade); sem expurgo por data.')
ON CONFLICT (dataset) DO NOTHING;
