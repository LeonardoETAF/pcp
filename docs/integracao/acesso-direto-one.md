# Acesso direto (somente leitura) ao One → PCP independente

> **Decisão (2026-06-16):** o One é **PostgreSQL 9.5.17** (`SELECT version()`), **abaixo do 10** →
> **replicação lógica é inviável** (existe só a partir do PG 10; CDC/Debezium idem, 9.5 fora de
> suporte). A aquisição será por **acesso direto somente-leitura** (consulta incremental) + **dump
> inicial** para o histórico. Substitui a abordagem de replicação (doc anterior descartado).
>
> O banco do One é **fonte exclusiva**; o banco do PCP é **independente** (VPS). Mapeamento de
> campos: [mapeamento-one-para-pcp.md](mapeamento-one-para-pcp.md).

## 1. Conectividade VPS ↔ LAN (ponto crítico de segurança)

O One está numa **LAN** (`192.168.88.251:5432`, IP privado) e o PCP roda numa **VPS** remota.
A VPS **não alcança** um IP privado diretamente, e **NÃO se deve expor o 9.5 (EOL) na internet**.
Duas formas seguras (decisão registrada em §6):

- **(B) Conector on-premise — ADOTADO:** um **agente nosso roda DENTRO da LAN** (lê o One
  localmente, read-only) e **envia** os dados para a VPS por **saída TLS** (push). O One **nunca**
  recebe conexão de fora — zero exposição do banco legado.
- **(A) VPN site-to-site / VPS como cliente VPN (alternativa):** a VPS entra na LAN por túnel e
  consulta `192.168.88.251` como se fosse local. Operação mais simples (deploy único na VPS), mas
  abre a direção de entrada.

> Em ambos, a regra de ouro: **conexão somente-leitura ao One, TLS, segredos em variável de
> ambiente** (CLAUDE.md §7). Jamais publicar a porta do 9.5 na internet, mesmo com firewall.

## 2. Arquitetura (independente do método de conectividade)

```
One (PG 9.5, LAN, READ-ONLY)
   │  ETL: SELECT incremental (a cada N min) + dump inicial (backfill)
   ▼
PCP (VPS): schema "bronze" (cru do One)  →  ACL (transforma)  →  domínio limpo (pcp)
```

- **Fronteira `FonteDados` (CLAUDE.md §1/§8):** nova implementação **`FonteConsultaOne`** (SQLx
  read-only, pull incremental) — substitui o conceito de `FonteReplicaOne`. O `ImportadorArquivo`
  (CSV/dump) segue para o backfill. **O motor e o domínio não mudam.**
- **Camada anticorrupção:** `bronze` (formato cru do One) → transforma para `vendas_dia`/
  `estoque_snapshot` (nomes honestos, tipos certos). Quando o One sair, o `bronze` some; o domínio
  permanece.

## 3. Estratégia de extração por tabela (9.5 + tabelas grandes)

Polling não captura DELETE nem UPDATE silencioso → estratégia por natureza do dado:

| Dado | Tabela | Estratégia |
|---|---|---|
| **Estoque** (snapshot) | `F03005` | **Full refresh** a cada ciclo — é o saldo atual e são poucos milhares de produtos (barato). |
| **Vendas/pedidos** | `F05001`+`F05002` | **Incremental por data** (`PEDV_DATC` > marca-d'água) **+ re-leitura de janela deslizante** (ex.: últimos 7–15 dias) para capturar **cancelamentos** (`ITMP_STPD`→`CANCELADO`) e alterações de pedidos recentes. |
| **Faturas** | `F10901`+`F10911` | Incremental por `FAT_DTEMI` + janela deslizante (status `FAT_STFAT`). |
| **Produção** | `F06002`/`F060015`/`F06018` | Incremental por data de produção/programação + janela deslizante p/ status. |
| **Produtos** | `F03001` | Full refresh periódico (catálogo pequeno; sem coluna de update confiável). |

- **Exigência:** as colunas de data usadas como marca-d'água (`PEDV_DATC`, `FAT_DTEMI`, …) precisam
  ter **índice** no One — senão a query incremental vira *full scan* numa tabela de 14M linhas.
  Confirmar com o suporte (ou checar no DBeaver).
- **Cadência:** tabelas quentes (pedidos, estoque) a cada **1–5 min** (quase tempo real); catálogo
  e janelas de backfill em horários de baixa carga.
- **Idempotência por data** preservada (igual ao `ImportadorArquivo`).

## 4. Tempo real — expectativa realista

Com 9.5 e sem mexer no One, **não há tempo real estrito**. O polling entrega **quase tempo real**
(latência = intervalo, ex.: 1–2 min). Para um PCP (motor em ciclo) isso é adequado. A atualização
da UI usa o canal **SSE/LISTEN-NOTIFY** que já existe, disparado ao fim de cada ciclo de ingestão.

## 5. O que pedir ao suporte do One (mínimo — ver pedido-suporte-one.md)

Encolheu muito (não há `wal_level`/publicação/slot/restart):
1. **Usuário somente-leitura** (`GRANT SELECT`) nas tabelas mapeadas.
2. **Acesso de rede:** liberar a **máquina-ponte (conector)** na LAN a ler o One localmente
   (read-only). Não precisa abrir nada para fora (o conector só faz saída TLS para a VPS).
3. Confirmar **índices** nas colunas de data usadas como marca-d'água.

## 6. Conectividade — DECISÃO (2026-06-16)

**Produção:** **conector on-premise (push de saída)**, rodando numa **máquina-ponte dedicada** na
LAN (o usuário já tem uma máquina reservada para isso). Vence em **segurança E desempenho** — não
há trade-off entre os dois:

- **Segurança:** o One **nunca** recebe conexão de fora; sem porta de entrada na LAN; a credencial
  do One **fica na LAN**. Se a VPS for comprometida, o invasor **não alcança** o One nem a rede
  (menor raio de dano) — postura ideal para um banco **EOL (9.5)**.
- **Desempenho:** as consultas ao banco são **locais** (LAN gigabit, ~0,1 ms) em vez de cruzarem a
  internet a cada round-trip; só o **dado necessário, filtrado e comprimível**, vai para a VPS.
  Ganho real no **backfill** e em queries pesadas; no polling leve do dia a dia, empata com VPN.

Reforços da máquina-ponte: dedicada (sem outros serviços), SO atualizado, firewall, idealmente em
segmento que só enxergue o One; usuário read-only; TLS; segredos em env.

**Alternativa (não adotada):** **VPN** (WireGuard na máquina dedicada + **ACL** travando a VPS para
alcançar só `192.168.88.251:5432`). Operação mais simples (ETL inteiro na VPS), desempenho
equivalente só no tráfego leve, mas abre a direção de entrada na LAN.

**Desenvolvimento (agora):** **conexão local direta** — o dev está na mesma rede do One; nada de
VPN/conector é necessário nesta fase.

- Onde roda o agente de ETL: **na máquina-ponte da LAN** (conector). Na VPS só ficaria se fosse VPN.
