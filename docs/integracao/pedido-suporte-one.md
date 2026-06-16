# Pedido ao suporte do One — habilitar replicação lógica (somente leitura)

> **Objetivo:** liberar a leitura dos dados do One em tempo real, por **replicação lógica nativa
> do PostgreSQL**, para um banco nosso (independente). **Não alteramos schema nem a aplicação do
> One** — são ajustes de **infraestrutura** + um usuário **somente-leitura**. Nós só *assinamos*;
> o One nunca escreve no nosso lado. Único impacto: `wal_level=logical` exige **um restart** do
> PostgreSQL (combinar janela).

## 1. Parâmetros do servidor (`postgresql.conf`)
```conf
wal_level = logical            # exige restart do PostgreSQL
max_wal_senders = 10           # ou (valor atual + 2)
max_replication_slots = 10     # ou (valor atual + 2)
```

## 2. Usuário de replicação — SOMENTE LEITURA
```sql
CREATE ROLE pcp_repl WITH LOGIN REPLICATION PASSWORD '<defina-uma-senha-forte>';
GRANT USAGE ON SCHEMA prd TO pcp_repl;
GRANT SELECT ON
  prd.F03001, prd.F03005,                 -- produto, estoque
  prd.F05001, prd.F05002,                 -- itens de pedido + cabeçalho (vendas/demanda)
  prd.F10901, prd.F10911,                 -- faturas: cabeçalho + item (venda faturada)
  prd.F06002, prd.F060015, prd.F06018     -- produção (WMS): item, prod×pedido, programação
  -- + o cabeçalho de produção (lote), código a confirmar (provável F06001)
  TO pcp_repl;
```

## 3. Publicação (somente as tabelas/colunas que usamos)
```sql
-- PostgreSQL >= 15 (com lista de colunas — trafega só o necessário):
CREATE PUBLICATION pcp_pub FOR TABLE
  prd.F03001 (ITM_ID, ITM_DESC, ITM_SKU, ITM_REF, ITM_GPPRD, ITM_TP, ITM_ATIV, ITM_PRODA),
  prd.F03005 (EST_ID, EST_ITM, EST_DCONF, EST_QTDE, EST_QTDR, EST_QTDD, EST_QTEM, EST_FLIN),
  prd.F05001 (ITMP_ID, ITMP_PEDV, ITMP_PRD, ITMP_QNT, ITMP_CONF, ITMP_STPD, ITMP_DCAN),
  prd.F05002 (PEDV_ID, PEDV_DATC, PEDV_DCAN, PEDV_DATA, PEDV_DTAP),
  prd.F10901 (FAT_ID, FAT_NUMERO, FAT_STFAT, FAT_DTEMI, FAT_DTSAI),
  prd.F10911 (FTI_ID, FTI_FATURA, FTI_PROD, FTI_CONF, FTI_QTDE, FTI_DPRD),
  prd.F06002 (IPRD_ID, IPRD_LOTE, IPRD_PRD, IPRD_QNT, IPRD_QNTT, IPRD_STAT),
  prd.F060015 (PPD_ID, PPD_LOTE, PPD_PEDV, PPD_QTDPRD),
  prd.F06018 (PRP_ID, PRP_IPRD, PRP_STPRD, PRP_DATP);
  -- + cabeçalho de produção (lote) quando confirmarem o código (provável prd.F06001)

-- PostgreSQL < 15 (sem lista de colunas — publicar as tabelas inteiras):
-- CREATE PUBLICATION pcp_pub FOR TABLE prd.F03001, prd.F03005, prd.F05001, prd.F05002,
--   prd.F10901, prd.F10911, prd.F06002, prd.F060015, prd.F06018;
```

## 4. Identidade de réplica (para UPDATE/DELETE chegarem)
As tabelas têm PK, então o padrão já basta. PKs (campo `..._ID` de cada tabela):
`F03001.ITM_ID` · `F03005.EST_ID` · `F05001.ITMP_ID` · `F05002.PEDV_ID` · `F10901.FAT_ID` ·
`F10911.FTI_ID` · `F06002.IPRD_ID` · `F060015.PPD_ID` · `F06018.PRP_ID`.
Se alguma tabela **não** tiver PK: `ALTER TABLE prd.Fxxxxx REPLICA IDENTITY FULL;`

## 5. Rede e segurança (`pg_hba.conf`)
```conf
# Liberar SÓ o IP do nosso assinante, sempre por TLS (hostssl) e scram:
hostssl  all          pcp_repl  <IP_DO_NOSSO_SERVIDOR>/32  scram-sha-256
hostssl  replication  pcp_repl  <IP_DO_NOSSO_SERVIDOR>/32  scram-sha-256
```
- Abrir a porta do PostgreSQL **apenas** para o nosso IP, idealmente via **VPN**.
- **TLS obrigatório.**

## 6. O que precisamos que vocês nos informem de volta
1. **Versão do PostgreSQL** do One (define se usamos lista de colunas — item 3).
2. **Host/porta** do banco e a forma de conectividade (VPN? liberar IP?).
3. Confirmar que **as colunas do item 3 existem** com esses nomes — em especial
   **`F03001.ITM_PRODA`** (flag de "personalizado") e os campos das faturas (`F10901`/`F10911`)
   e de produção (`F06002`/`F060015`/`F06018`).
4. **Código da tabela de cabeçalho de produção (lote)** — entidade `Producao` (provável `F06001`).
5. Confirmar que essas tabelas têm **PK**.

## 7. Observação operacional (combinar conosco)
O *slot* de replicação acumula WAL no One **se o nosso assinante ficar offline por muito tempo**.
Nós monitoramos o atraso (lag) e mantemos o assinante saudável; se precisarem derrubar a
publicação por qualquer motivo, é só avisar.

---

### Tabelas e por que precisamos de cada uma
| Tabela | Uso no PCP |
|---|---|
| `prd.F03001` (Produto) | nome/SKU, grupo (filtrar PRODUTO_ACABADO), flag de personalizado (`ITM_PRODA`) |
| `prd.F03005` (Estoque) | snapshot de estoque (saldo global, disponível, fora de linha, configuração) |
| `prd.F05001` (ItemPedido) | **vendas/demanda** = itens de pedido **não cancelados** (`ITMP_STPD <> 'CANCELADO'`) |
| `prd.F05002` (PedidoVenda) | data do pedido (`PEDV_DATC`) e cancelamento do cabeçalho |
| `prd.F10901` + `prd.F10911` (Fatura) | **venda faturada** (cabeçalho + item) — substitui a NF, pois há vendas sem NF |
| `prd.F06002` (ItemProducao) | **produção**: item em produção (produto, qtd, status) |
| `prd.F060015` (ProducaoPedido) | liga produção ↔ pedido (qtd a produzir) |
| `prd.F06018` (ProgramacaoProducao) | programação de produção (setor, data) |
| `prd.F06001`? (Producao) | cabeçalho do lote de produção — **confirmar código** |
| `prd.F03007` / `prd.F10011` | *(opcional)* só verificação cruzada (Cardex / NF) |
