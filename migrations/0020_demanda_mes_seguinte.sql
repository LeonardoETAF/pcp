-- Variável nova: DEMANDA DO MÊS SEGUINTE NO ANO PASSADO (decisão do dono, 2026-07-13).
--
-- A produção de hoje serve o mês que vem. Saber o que o produto vendia nesta mesma época, um ano
-- atrás, é o melhor sinal antecipado que temos — e é por produto, não pela curva da empresa.
--
-- ATENÇÃO à armadilha: um produto que vendeu ZERO no mês seguinte do ano passado pode ser um
-- produto que AINDA NÃO EXISTIA (o 23154 nasceu em set/2025; agosto/2025 é zero por isso, não por
-- ser um mês morto). Por isso a coluna é NULL-able: NULL = "não aplicável, o produto não existia";
-- 0 = "existia e não vendeu". São coisas diferentes e o motor não pode confundi-las.
ALTER TABLE pcp.estoque_param
    ADD COLUMN demanda_mes_seguinte double precision;

COMMENT ON COLUMN pcp.estoque_param.demanda_mes_seguinte IS
    'Média diária (dias corridos) do mês seguinte no ano passado. NULL = produto não existia lá.';
