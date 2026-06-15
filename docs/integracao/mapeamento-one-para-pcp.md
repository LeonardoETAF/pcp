# Mapeamento ERP One → contrato do PCP

> Baseado na **camada de persistência do One** (entidades JPA/Hibernate — `persistence.rar`,
> sistema **SGTA / BusinessView**). As tabelas têm nomes em código (`F03001`, `F03005`…), no
> schema **`prd`** (visto nos comentários do código, ex.: `prd.F03001`). Aqui ligamos cada
> campo do One ao contrato de entrada do PCP de [contrato-dados-one.md](contrato-dados-one.md).
>
> ⚠️ Este arquivo é o modelo de **estrutura** (entidades), **não** os dados. Os registros
> (linhas) ainda serão necessários para rodar o motor.

## Tabelas-chave do One identificadas

| Tabela | Entidade | Papel |
|---|---|---|
| `F03001` | `Produto` (abstrata; `ProdutoSimples`/`ProdutoConfiguravel`) | cadastro do produto |
| `F03005` | `Estoque` | **saldo atual** por produto × configuração |
| `F03007` | `Cardex` | **livro-razão de estoque** (movimentos com data) |
| `F030022` | `TipoPersonalizacao` | catálogo de tipos de personalização |
| `F05002` | `PedidoVenda` | cabeçalho do pedido de venda (datas, cliente) |
| `F05001` | `ItemPedido` | item do pedido de venda |
| `F10011` | `NotaFiscalItem` | item da nota fiscal (faturamento) |

---

## 1. ESTOQUE / snapshot (`estoque.csv`) ← `F03005` (+ `F03001`)

A `Estoque` (F03005) entrega quase tudo direto. Granularidade no One é **produto × configuração**;
o PCP quer **uma linha por produto** → somar as configurações (e `fora_de_linha` por `BOOL_OR`).

| Campo PCP | Origem no One | Observação |
|---|---|---|
| `codigo_estoque` | `F03005.EST_ITM` → `F03001.ITM_ID` | id do produto (chave) |
| `sku` | `F03001.ITM_SKU` | |
| `produto` | `F03001.ITM_DESC` | descrição/nome |
| `configuracao` | `F03005.EST_DCONF` (`descricaoConfiguraveis`) | texto pronto da variação |
| `qtd_estoque` | `F03005.EST_QTDE` | `Double` → arredondar p/ inteiro (unidades) |
| `qtd_reserva` | `F03005.EST_QTDR` | reserva firme |
| `qtd_disponivel` | **`F03005.EST_QTDD`** | ✅ **suporte: usar SEMPRE o "saldo disponível"** |
| `estoque_min_erp` | `F03005.EST_QTEM` | só referência |
| `fora_de_linha` | **`F03005.EST_FLIN`** | flag direta ✓ (resolve P6) |

✅ **Confirmado pelo suporte:** `F03005` é o **saldo de estoque GERAL (global)** — estoque por
depósito só existe dentro do WMS; para o PCP, o saldo global é exatamente o que queremos (não há
tratamento de almoxarifado). E o "disponível" canônico é o `EST_QTDD` (usar sempre).

Filtro: apenas `F03001.ITM_GPPRD = 'PRODUTO_ACABADO'` (enum `GrupoProduto`). **F03005 tem uma linha
por produto mesmo com saldo zero** → satisfaz o "snapshot completo" (P5). `F03001.ITM_ATIV`
(`enable`) é "ativo no cadastro" — conceito **diferente** de fora de linha; usar `EST_FLIN`.

---

## 2. VENDAS (`vendas.csv`) — RESOLVIDO (P1): pedidos não cancelados

✅ **Resposta do suporte:** "**venda = pedidos `F05001` que não estejam cancelados**". O One
distingue três estágios do pedido (úteis se quisermos refinar a paridade):
- **Pedido (venda)** — itens `F05001` **não cancelados** ← **definição adotada**.
- **Autorizado/pago** — `F05002` com a **data de autorização preenchida** (`PEDV_DATA`; pagamento `PEDV_DTAP`).
- **Faturado** — tabela de **Faturas** (`F10990`? — o suporte não tinha certeza do código; confirmar só se formos por aqui).

Adotamos **pedidos não cancelados (Opção B)**. As opções A (NF) e C (Cardex) ficam como
referência cruzada.

### Opção A — Faturamento / Nota Fiscal — `F10011` + cabeçalho NF
Venda **faturada** (saiu nota). Verificação cruzada.

| Campo PCP | Origem |
|---|---|
| `dt_ref` | data de emissão da NF (cabeçalho `NotaFiscal`/`NotaFiscalVenda`) |
| `codigo_estoque` | `F10011.NFI_PROD` (→ `ITM_ID`) ou `NFI_CPRD` (código impresso) |
| `produto` | `F10011.NFI_DPRD` |
| `qtd_vendida` | `F10011.NFI_QTDE` (descontar `NFI_QTDD` = devolvida) |
| `configuracao` | (vem do item de pedido vinculado / `descricaoConfiguraveis`) |
| valor (futuro) | `F10011.NFI_UNIT` (custo/preço — dimensão financeira) |

### ✅ Opção B (ADOTADA) — Pedidos de venda — `F05001` (item) + `F05002` (cabeçalho)

| Campo PCP | Origem |
|---|---|
| `dt_ref` | `F05002.PEDV_DATC` (`dataPedido`) |
| `codigo_estoque` | `F05001.ITMP_PRD` (→ `ITM_ID`) |
| `qtd_vendida` | `F05001.ITMP_QNT`, **excluindo cancelados** (`ITMP_DCAN` nulo / `ITMP_STPD` ≠ cancelado; e pedido com `PEDV_DCAN` nulo) |
| `configuracao` | `F05001.ITMP_CONF` (`descricaoConfiguraveis`) |
| `is_personalizado` | `F03001.ITM_PRODA` (atributo do produto — §3) |

Consolidação: somar `qtd_vendida` por (`dt_ref`, produto) — variações entram como linhas e o motor
as soma (doc 02 §1).

### Opção C — Saídas do Cardex — `F03007` (`Cardex`)
Movimentos de estoque do tipo saída/venda. Boa para casar com o estoque real.

| Campo PCP | Origem |
|---|---|
| `dt_ref` | `F03007.CDX_DATC` (`dataMovimentacao`) |
| `codigo_estoque` | `F03007.CDX_ITM` |
| `qtd_vendida` | `F03007.CDX_QTD` onde `CDX_TPMVM` (`TipoMovimentacaoEstoque`) = venda/saída |
| `configuracao` | `F03007.CDX_DSCF` |

**Decisão (P1):** venda = **Opção B — pedidos `F05001` não cancelados** (resposta do suporte).
A/C ficam só como verificação cruzada. Validar a paridade com `6797`/`10001`/`10473` + a
distribuição (doc 08 §3) antes do cut-over; se não bater, alternar para "autorizados" (`PEDV_DATA`)
ou "faturados" (`F10990`) — o caminho de cálculo dos três já está mapeado.

---

## 3. `is_personalizado` — RESOLVIDO (P2)

✅ **Definição oficial do suporte:** personalizado = produto que pode ser customizado com
**estampa ou borda**, indicado pelo atributo **`F03001.ITM_PRODA`** (booleano no cadastro do
produto). Ou seja, `is_personalizado` é uma **propriedade do produto**, não da linha de venda:

```
is_personalizado(venda) = F03001.ITM_PRODA  (do produto vendido)
```

(Os sinais antes cogitados — `urlArte`, `tipoPersonalizacao` — ficam como contexto; a fonte
canônica é `ITM_PRODA`.)

## 4. `configuracao` (variação) — RESOLVIDO (P3)

✅ **Confirmado pelo suporte:** `descricaoConfiguraveis` (`EST_DCONF`, `ITMP_CONF`, `CDX_DSCF`) **já
vem sempre no padrão `"CHAVE: valor"`**, podendo ter 1+ atributos (às vezes 3–4, ex.:
`COR DO PRODUTO: AZUL | NOME DA ESTAMPA: X | COR DA TAMPA: Y`). Para o PCP basta consumir esse
texto direto.

## 5. Histórico (bônus importante)

- **Estoque histórico:** `F03005` é só o saldo **atual**. Mas o **Cardex `F03007`** registra cada
  movimento com data e saldo (`CDX_SDAT`), então dá para **reconstruir o snapshot de qualquer data
  passada** — resolve a antiga limitação de "snapshot só do dia".
- **Vendas histórico:** conforme a fonte escolhida (NF/Pedido/Cardex), extrair **24 meses** (ABC
  18m + sazonalidade do ano anterior). Ver janelas em [contrato-dados-one.md](contrato-dados-one.md) §4.

## 6. Dimensão financeira (futura, opcional)

Disponível para quando entrar: `F10011.NFI_UNIT` (preço de venda na NF), entidades
`product/Preco.java`, `product/PrecoCusto.java`, `ItemPedido.ITMP_VUNIT`.

---

## 7. Respostas às perguntas P1–P9

| # | Pergunta | Situação |
|---|---|---|
| P1 | O que é "venda"/qual data | ✅ **pedidos `F05001` não cancelados** (data `PEDV_DATC`). Ver §2. |
| P2 | Como marca personalizado | ✅ `F03001.ITM_PRODA` (produto customizável c/ estampa/borda). |
| P3 | Variação/configuração | ✅ `descricaoConfiguraveis` já vem `"CHAVE: valor"` (1+ atributos). |
| P4 | Chave do produto | ✅ `F03001.ITM_ID` (mesma FK em estoque/pedido/cardex/NF). |
| P5 | Snapshot completo (incl. zero/fora) | ✅ `F03005` tem linha por produto mesmo com saldo 0 + `EST_FLIN`. |
| P6 | Fora de linha | ✅ `F03005.EST_FLIN`. |
| P7 | Depósitos/almoxarifado | ✅ `F03005` = saldo **global**; por depósito só no WMS (não usamos). |
| P8 | Reserva | ✅ usar **`EST_QTDD`** (saldo disponível) sempre. |
| P9 | Entrega recorrente/API | → resolvido pela **replicação lógica** ([replicacao-tempo-real.md](replicacao-tempo-real.md)). |

## 8. Mapeamento completo — confirmações menores pendentes

O mapeamento funcional está **fechado** (P1–P9 resolvidos). Restam só confirmações que **não**
bloqueiam a implementação pela definição adotada (pedidos não cancelados):

- **Paridade:** validar os números contra `6797`/`10001`/`10473` + a distribuição (doc 08 §3)
  quando houver dados; se "pedidos não cancelados" não bater, alternar para autorizados/faturados.
- **Só se formos por "autorizado":** confirmar se a data é `PEDV_DATA` (aprovação) ou `PEDV_DTAP` (pagamento).
- **Só se formos por "faturado":** confirmar o código real da tabela de Faturas (`F10990`?).

Faltam ainda os pré-requisitos de **infraestrutura** da replicação (não do mapeamento): versão do
PostgreSQL do One, PKs, provisionar o staging e a conectividade — ver
[replicacao-tempo-real.md](replicacao-tempo-real.md) Parte C.
