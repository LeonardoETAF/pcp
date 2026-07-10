-- O universo de planejamento passa a ser só o produto LISO (ITM_PRODA = false).
--
-- Por quê: o personalizado é feito sob encomenda. Ele não guarda estoque — das 20.532 linhas
-- personalizadas, 19.539 têm disponível zero, e o saldo agregado é NEGATIVO (−173.944 un, a
-- origem do "disponível negativo" que aparecia na tela). Quando um personalizado é pedido, o One
-- **reserva a linha do liso** correspondente (confirmado pelo suporte). Quem se planeja, produz e
-- tem cobertura é o liso.
--
-- Consistente com o PRD: os três produtos de referência da §11 (6797, 10001, 10473) são todos
-- lisos, e o universo cai de 23.946 para 3.414 linhas — na ordem das 2.380 que o legado
-- classificava.
--
-- Nenhuma demanda é perdida: no kardex cada linha tem os seus próprios movimentos de VENDA, então
-- excluir o personalizado não subtrai venda do liso.
--
-- O bronze e as derivadas são re-deriváveis do One (ver 0013/0015); o backfill e o motor repovoam.

TRUNCATE bronze.one_estoque, bronze.one_venda;
TRUNCATE pcp.vendas_dia, pcp.estoque_snapshot,
         pcp.classificacao, pcp.estoque_param, pcp.alerta, pcp.sugestao_ciclo_vida
    RESTART IDENTITY CASCADE;
DELETE FROM bronze.sincronizacao WHERE fonte = 'vendas';
