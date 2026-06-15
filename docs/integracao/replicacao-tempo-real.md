# Replicação lógica em tempo real: One (publicador) → PCP (assinante)

> Objetivo: dados do One **quase em tempo real** num banco **independente**, sem alterar schema
> nem aplicação do One (só config de infraestrutura). Mecanismo: **replicação lógica nativa do
> PostgreSQL**. Premissas e comparação: [aquisicao](mapeamento-one-para-pcp.md) e o histórico de
> decisão. Contrato de campos: [contrato-dados-one.md](contrato-dados-one.md).

---

## PARTE A — Pedido técnico para o suporte do One (lado publicador)

> Nada aqui altera o **schema** nem a **aplicação** do One. São ajustes de **infraestrutura** do
> PostgreSQL + um usuário somente-leitura. O `wal_level` exige **um restart** (janela combinada).

### A.1 Parâmetros do servidor (`postgresql.conf`)
```conf
wal_level = logical            # exige restart do PostgreSQL
max_wal_senders = 10           # ou (valor atual + 2)
max_replication_slots = 10     # ou (valor atual + 2)
```

### A.2 Usuário de replicação **somente-leitura**
```sql
CREATE ROLE pcp_repl WITH LOGIN REPLICATION PASSWORD '<senha-forte>';
GRANT USAGE ON SCHEMA prd TO pcp_repl;
-- Apenas as tabelas que o PCP consome (ajustar a fonte de VENDA conforme decisão P1):
GRANT SELECT ON prd.F03001, prd.F03005, prd.F03007,
                prd.F05001, prd.F05002, prd.F10011 TO pcp_repl;
```

### A.3 Publicação (somente as tabelas necessárias)
```sql
-- PostgreSQL >= 15 (permite listar só as colunas necessárias — menos dado trafegado):
CREATE PUBLICATION pcp_pub FOR TABLE
  prd.F03001 (ITM_ID, ITM_DESC, ITM_SKU, ITM_REF, ITM_GPPRD, ITM_TP, ITM_ATIV),
  prd.F03005 (EST_ID, EST_ITM, EST_DCONF, EST_QTDE, EST_QTDR, EST_QTDD, EST_QTEM, EST_FLIN),
  prd.F03007 (CDX_ITM, CDX_DSCF, CDX_QTD, CDX_TPMVM, CDX_DATC);
  -- + a fonte de VENDA escolhida (P1): F10011 (NF) e/ou F05001+F05002 (pedido).

-- PostgreSQL < 15: publicar as tabelas inteiras (sem lista de colunas):
-- CREATE PUBLICATION pcp_pub FOR TABLE prd.F03001, prd.F03005, prd.F03007, ... ;
```

### A.4 Identidade de réplica (para UPDATE/DELETE chegarem)
- Cada tabela publicada precisa de **chave primária** (têm: `ITM_ID`, `EST_ID`, …) — então o
  `REPLICA IDENTITY` padrão (PK) basta.
- Se alguma tabela **não** tiver PK, definir: `ALTER TABLE prd.Fxxxxx REPLICA IDENTITY FULL;`

### A.5 Rede e segurança (`pg_hba.conf`)
```conf
# Liberar o IP do nosso assinante, sempre por TLS (hostssl) e scram:
hostssl  all          pcp_repl  <IP_ASSINANTE>/32  scram-sha-256
hostssl  replication  pcp_repl  <IP_ASSINANTE>/32  scram-sha-256
```
- Abrir a porta do PostgreSQL do One **apenas** para o IP do assinante, **idealmente via VPN**.
- **TLS obrigatório** (`sslmode=require`/`verify-full`).

### A.6 Informações que precisamos do suporte
1. **Versão do PostgreSQL** do One (define se usamos lista de colunas na publicação — A.3).
2. Confirmar **PK** em cada tabela publicada (A.4).
3. **Host/porta/IP** e a forma de conectividade (VPN?).
4. Decisão **P1** (qual tabela é a "venda" canônica — ver contrato §8).

> **Impacto operacional:** o slot de replicação acumula WAL no One **se o assinante ficar offline
> por muito tempo** → nós monitoramos o atraso do slot (lag) e mantemos o assinante saudável.

---

## PARTE B — Arquitetura do lado do PCP (assinante)

```
┌──────────── One (Postgres, schema prd) ────────────┐
│  PUBLICATION pcp_pub  (F03001, F03005, F03007, …)   │   somente-leitura, TLS/VPN
└───────────────────────┬─────────────────────────────┘
                        │  replicação lógica (WAL, push, ~tempo real)
                        ▼
┌──────── BANCO DE STAGING (independente do PCP) ─────┐   "bronze" — formato cru do One
│  SUBSCRIPTION pcp_sub → tabelas espelho do One      │   ninguém de negócio lê isto
└───────────────────────┬─────────────────────────────┘
                        │  Camada anticorrupção (ETL, async/SQLx) — nomes honestos, tipos certos
                        ▼
┌──────── BANCO DO PCP (dedicado — CLAUDE.md §6) ─────┐   "gold" — domínio limpo/performático
│  vendas_dia · estoque_snapshot · produto_ativo …    │   é a fonte da verdade do novo sistema
└──────────────────────────────────────────────────────┘
```

### B.1 Decisões de arquitetura (alinhadas ao CLAUDE.md)
- **Staging é um banco SEPARADO** do banco do PCP (não misturar tabelas do One no banco do PCP —
  §6 "Postgres dedicado, só do PCP"). Pode ser outro *database* no mesmo servidor ou instância à
  parte; **decisão de infraestrutura a confirmar**.
- **Fronteira `FonteDados` preservada (§1/§8):** a replicação alimenta o staging; uma **nova
  implementação assíncrona de `FonteDados`** (`FonteReplicaOne`) lê do staging via SQLx e
  transforma para `NovaVendaDia`/`NovoEstoqueSnapshot`. **O motor não muda.** O
  `ImportadorArquivo` (CSV/dump) **permanece** para o backfill inicial e testes.
- **Camada anticorrupção (ACL):** é onde `EST_QTDE`/`EST_FLIN`/`EST_DCONF` viram
  `qtd_estoque`/`fora_de_linha`/`configuracao`, com tipos corretos (Double→inteiro de unidades) e
  consolidação produto×configuração → 1 linha por produto.
- **Tempo real no PCP:** a replicação mantém o staging fresco continuamente; o canal
  **LISTEN/NOTIFY + SSE** que já existe propaga "dados novos" para a UI. O cálculo pesado do
  motor segue diário (idempotente); leituras frescas vêm do staging/derivadas.
- **Segurança (§7):** conexão ao One **somente-leitura**; credenciais só em variável de ambiente;
  TLS; staging isolado; usuário final nunca acessa staging nem o One; escrita só pelo pipeline.
- **Dependência de mão única:** o PCP **assina/puxa**; o One nunca conhece nem escreve no PCP.

### B.2 Esboço da assinatura (lado staging)
```sql
CREATE SUBSCRIPTION pcp_sub
  CONNECTION 'host=<one> port=5432 dbname=<one_db> user=pcp_repl sslmode=verify-full'
  PUBLICATION pcp_pub;
```

---

## PARTE C — O que falta para implementar o código (bloqueios reais)

A implementação do `FonteReplicaOne` + staging + ACL depende de definições que ainda não temos —
escrever agora seria chute (e o CLAUDE.md proíbe mock que finge dado real, §13):

1. **P1 — fonte de "venda" não decidida** (NF `F10011` × Pedido `F05002` × Cardex `F03007`).
   Define a publicação **e** a transformação de vendas. (Estoque via `F03005` já está claro.)
2. **Versão do PostgreSQL do One** (A.6) e **PKs** (A.4).
3. **Infra do staging:** banco separado no mesmo servidor ou instância dedicada? (define
   `STAGING_DATABASE_URL`, pool, docker-compose).
4. **Conectividade** (IP/VPN) para a assinatura subir.

### Plano de implementação (assim que desbloqueado)
1. Trait de fonte **assíncrona** (`FonteDados` async ou irmão `FonteDadosAsync`) + `importar` async.
2. `FonteReplicaOne`: lê do staging (SQLx, read-only) e aplica a ACL → contrato.
3. Migrations do **staging** (tabelas espelho do One — subset publicado).
4. Gatilho de **tempo real** ligado ao NOTIFY/SSE existente.
5. Testes da ACL com staging sintético (fixtures), validando os produtos 6797/10001/10473.
