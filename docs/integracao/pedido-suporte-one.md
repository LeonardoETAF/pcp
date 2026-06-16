# Pedido ao suporte do One — acesso de leitura para o PCP

> **Contexto:** o One é PostgreSQL **9.5.17**, então **não usaremos replicação** (ela exige PG ≥ 10).
> A integração será por **acesso somente-leitura** (consultas), feito pelo nosso sistema, que roda
> numa **VPS**. **Não alteramos nada no One** (sem mudança de schema, config ou restart). Detalhes
> da arquitetura: [acesso-direto-one.md](acesso-direto-one.md).

## 1. Usuário de banco SOMENTE LEITURA
```sql
CREATE ROLE pcp_ro WITH LOGIN PASSWORD '<defina-uma-senha-forte>';
GRANT USAGE ON SCHEMA prd TO pcp_ro;
GRANT SELECT ON
  prd.F03001, prd.F03005,               -- produto, estoque
  prd.F05001, prd.F05002,               -- itens de pedido + cabeçalho (vendas)
  prd.F10901, prd.F10911,               -- faturas: cabeçalho + item (venda faturada)
  prd.F06002, prd.F060015, prd.F06018   -- produção (WMS): item, prod×pedido, programação
  -- + o cabeçalho de produção (lote), código a confirmar (provável F06001)
  TO pcp_ro;
```
Sem permissão de escrita, DDL ou replicação — apenas `SELECT` nessas tabelas.

## 2. Acesso de rede (VPS → LAN)
O PCP roda numa **VPS** (remota) e o One está na **LAN** (`192.168.88.251:5432`). Como conectar,
**sem expor o banco na internet** (escolher uma):
- **VPN:** liberar a VPS (por VPN) a alcançar `192.168.88.251:5432`. **TLS obrigatório.**
- **ou Conector on-premise:** rodamos um agente leitor **dentro da rede de vocês**, que só faz
  conexões de **saída** para a nossa VPS (o One não recebe conexão externa nenhuma).

Indiquem qual é viável aí. **Não** abrir a porta do PostgreSQL diretamente na internet.

## 3. O que precisamos que vocês nos confirmem
1. Que as **colunas mapeadas existem** com esses nomes — em especial **`F03001.ITM_PRODA`** (flag de
   "personalizado"), os campos das faturas (`F10901`/`F10911`) e de produção (`F06002`/`F060015`/`F06018`).
2. **Código da tabela de cabeçalho de produção (lote)** — entidade `Producao` (provável `F06001`).
3. **Índices** nas colunas de data que usaremos para leitura incremental: `F05002.PEDV_DATC`,
   `F10901.FAT_DTEMI` (e datas de produção). Sem índice, a leitura incremental fica pesada nas
   tabelas grandes (ex.: `F10911` ~14M linhas).

## 4. Garantias do nosso lado
- Conexão **somente-leitura**; **nunca** escrevemos no One.
- Leitura **incremental** (só o que mudou) e **em horários de baixa carga** para o backfill, para
  não competir com o ERP.
- Senha em cofre/variável de ambiente; **TLS** na conexão.

---

### Tabelas e por que precisamos de cada uma
| Tabela | Uso no PCP |
|---|---|
| `prd.F03001` (Produto) | nome/SKU, grupo (filtrar PRODUTO_ACABADO), flag de personalizado (`ITM_PRODA`) |
| `prd.F03005` (Estoque) | snapshot de estoque (saldo global, disponível, fora de linha, configuração) |
| `prd.F05001` (ItemPedido) | **vendas/demanda** = itens de pedido **não cancelados** (`ITMP_STPD <> 'CANCELADO'`) |
| `prd.F05002` (PedidoVenda) | data do pedido (`PEDV_DATC`) e cancelamento do cabeçalho |
| `prd.F10901` + `prd.F10911` (Fatura) | **venda faturada** (cabeçalho + item) — substitui a NF |
| `prd.F06002` (ItemProducao) | **produção**: item em produção (produto, qtd, status) |
| `prd.F060015` (ProducaoPedido) | liga produção ↔ pedido (qtd a produzir) |
| `prd.F06018` (ProgramacaoProducao) | programação de produção (setor, data) |
| `prd.F06001`? (Producao) | cabeçalho do lote de produção — **confirmar código** |
