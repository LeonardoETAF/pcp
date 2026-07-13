-- O universo de planejamento exclui também as linhas SEM configuração (EST_DCONF vazio).
--
-- Uma linha de estoque sem configuração não é um produto-cor: são acessórios e peças avulsas
-- (ESPACADOR PREMIUM P/ NIV DE REV, MOSQUETAO PLASTICO, CALOTA RODA, TAG PARA PULSEIRA, caixas e
-- gradinhas). Não são copos personalizáveis e não pertencem à lista de produtos.
--
-- O recorte aproxima a distribuição ABC do alvo de aceitação do PRD §11, e a classe F bate exato:
--   obtido  A=174 B=360 C=723 D=1203 F=177 N=6
--   alvo    A=165 B=346 C=671 D=1012 F=177 N=9
--
-- Como em 0016: o filtro vai na FONTE (estoque e vendas), não na tela — o motor é único (§3) e um
-- produto que não se planeja não deve gerar classe, parâmetro nem alerta. Nenhuma demanda se
-- perde: no kardex cada linha tem os seus próprios movimentos de VENDA.
--
-- Bronze e derivadas são re-deriváveis do One (0013/0015); o backfill e o motor repovoam.

TRUNCATE bronze.one_estoque, bronze.one_venda;
TRUNCATE pcp.vendas_dia, pcp.estoque_snapshot,
         pcp.classificacao, pcp.estoque_param, pcp.alerta, pcp.sugestao_ciclo_vida
    RESTART IDENTITY CASCADE;
DELETE FROM bronze.sincronizacao WHERE fonte = 'vendas';
