# Contrato de dados a extrair do ERP One — para o PCP

> **Para quem dá suporte ao One.** Este documento lista **tudo** que o PCP precisa do banco
> do One para funcionar, campo a campo, com a profundidade de histórico necessária e o porquê
> de cada item. A integração começa por **arquivo (CSV)** — não precisamos de acesso direto ao
> banco agora; precisamos de **dois extratos** (vendas e estoque) no formato abaixo. Quando
> houver API do One, trocamos a fonte sem mexer no resto (o PCP lê tudo atrás de um contrato).
>
> Fontes desta especificação: PRD §02 (regras de negócio), §05 (contrato do ETL), §08 (legado).

---

## 1. Resumo executivo (o que precisamos)

São **dois conjuntos de dados**:

| Extrato | Granularidade | Quando | Profundidade |
|---|---|---|---|
| **A. Vendas** | 1 linha por **dia × produto × variação** | histórico inicial + diário | **24 meses** de histórico (mínimo 18) |
| **B. Estoque (snapshot)** | 1 linha por **produto**, foto do dia | snapshot atual + diário | foto **completa** do dia (todos os produtos) |

- **Carga inicial (uma vez):** vendas dos últimos **24 meses** + **um** snapshot completo de hoje.
- **Carga diária (recorrente):** vendas do **dia anterior** + snapshot **completo** do dia, todo
  dia de madrugada (o PCP processa e tem os números prontos até as 05:00).

A razão dos 24 meses está na seção 4 (as regras olham até 18 meses para trás, e a sazonalidade
precisa do ano-calendário anterior completo).

---

## 2. Extrato A — VENDAS (`vendas.csv`)

Uma linha **por dia, por produto e por variação**. Se um produto vendeu em duas variações no
mesmo dia (ex.: versão lisa + versão personalizada), são **duas linhas** — o PCP soma depois.

**Cabeçalho exato (nesta ordem):**

```
dt_ref,codigo_estoque,sku,produto,configuracao,qtd_vendida,is_personalizado
```

| Coluna | Tipo | Obrigatório | O que é / de onde sai no One |
|---|---|---|---|
| `dt_ref` | data `AAAA-MM-DD` | **sim** | dia da venda (ver pergunta P1: qual data — faturamento, saída?) |
| `codigo_estoque` | texto | **sim** | **código do produto/estoque** no One (chave única do produto) |
| `sku` | texto | não | SKU comercial (pode diferir entre versão lisa e personalizada) |
| `produto` | texto | não | descrição/nome do produto |
| `configuracao` | texto | não | variação no formato `"CHAVE: valor"` (ver seção 5) |
| `qtd_vendida` | inteiro ≥ 0 | **sim** | unidades vendidas naquele dia/variação |
| `is_personalizado` | `true`/`false` | **sim** | se a linha é de item **personalizado** (ver pergunta P2) |

Observações:
- **Append (acrescenta), não substitui** — exceto reprocesso explícito de uma data.
- Pode haver várias linhas por (`dt_ref`, `codigo_estoque`). Tudo bem.
- Não enviar valores monetários aqui (a dimensão financeira é tratada à parte — ver seção 6).

---

## 3. Extrato B — ESTOQUE / SNAPSHOT (`estoque.csv`)

Uma **foto completa** do estoque no dia: **uma linha por produto**, contendo **todos** os
produtos do catálogo — inclusive os **com estoque zero** e os **fora de linha** (são
necessários para a classificação; ver pergunta P5).

**Cabeçalho exato (nesta ordem):**

```
dt_ref,codigo_estoque,sku,produto,configuracao,qtd_estoque,qtd_reserva,qtd_disponivel,estoque_min_erp,fora_de_linha
```

| Coluna | Tipo | Obrigatório | O que é / de onde sai no One |
|---|---|---|---|
| `dt_ref` | data `AAAA-MM-DD` | **sim** | data da foto (o dia da extração) |
| `codigo_estoque` | texto | **sim** | mesmo código do produto usado nas vendas |
| `sku` | texto | não | SKU comercial |
| `produto` | texto | não | descrição/nome do produto |
| `configuracao` | texto | não | variação `"CHAVE: valor"` (seção 5) |
| `qtd_estoque` | inteiro | **sim** | quantidade **física total** em estoque |
| `qtd_reserva` | inteiro | **sim** | quantidade **reservada** (pedidos já comprometidos) |
| `qtd_disponivel` | inteiro | **sim** | `qtd_estoque − qtd_reserva` (o PCP valida; se faltar, calcula) |
| `estoque_min_erp` | inteiro | não | estoque mínimo cadastrado no One (só referência; o PCP recalcula o seu) |
| `fora_de_linha` | `true`/`false` | **sim** | produto descontinuado/inativo (ver pergunta P6) |

Observações:
- **Snapshot do dia substitui o snapshot daquele dia** (não é incremental; é a foto inteira).
- A invariante `qtd_disponivel = qtd_estoque − qtd_reserva` **precisa** bater (senão a linha é
  rejeitada). Se o One não tem o "disponível" pronto, pode deixar a coluna vazia que o PCP
  calcula — mas então `qtd_estoque` e `qtd_reserva` têm que estar corretos.

---

## 4. Profundidade de histórico — e por quê

Cada regra do PCP olha um período diferente das **vendas**. Por isso o histórico inicial:

| Regra (PRD §02) | Janela que ela usa |
|---|---|
| Classificação A/B/C (Pareto) | **540 dias (18 meses)** de vendas somadas |
| Classe D (sem giro) | últimos **180 dias** sem venda |
| Classe N (produto novo) | **1ª venda < 60 dias** |
| Parâmetros estatísticos (média, desvio, CV) | últimos **12 meses** (dias com venda) |
| Sazonalidade (fator por mês) | **ano-calendário anterior inteiro** de vendas |
| Ciclo de vida (fora de linha) | vendas 12m + recência (90/180/**365** dias) |

→ A janela mais longa que **manda** é 18 meses (ABC). Mas a **sazonalidade** compara cada mês
com o ano anterior completo e o ciclo de vida olha "≥ 365 dias sem vender". Para o sistema
nascer **calibrado** (fatores sazonais válidos desde o dia 1), o ideal é **24 meses** de vendas
na carga inicial. **Mínimo aceitável: 18 meses** (sazonalidade entra em regime no mês seguinte).

Para o **estoque**, o histórico não é crítico: basta **um snapshot completo de hoje** para
começar. O histórico de estoque vai sendo formado a partir das cargas diárias (usado depois só
para gráficos de tendência de 30 dias).

---

## 5. Campo `configuracao` (a variação do produto)

No legado, a variação vem como texto no padrão **`"CHAVE: valor"`**, com estas chaves conhecidas:

- `COR DO PRODUTO: <cor>`
- `NOME DA ESTAMPA: <nome>`
- `COR DA TAMPA: <cor>`
- `COR DO DEGRADE: <cor>`
- `COR DA METALIZACAO: <cor>`

O PCP usa isso só para **exibição** ("{produto} - {valor após os dois-pontos}"). Se no One a
variação/característica estiver em colunas separadas, o suporte precisa **montar essa string**
(ou nos dizer como as características são modeladas, que a gente adapta o mapeamento — ver P3).

---

## 6. Opcional agora, importante depois — dimensão financeira

O PCP **ainda não** usa custo/preço (a dimensão financeira foi adiada de propósito). Mas como o
suporte já vai mexer na extração, **se for barato**, vale capturar por produto:

- `custo_unitario` e `preco_unitario` (e a moeda).

Não bloqueia nada; só evita uma segunda rodada de extração quando formos calcular capital
parado / ROI. Se for trabalhoso, **ignore por enquanto**.

---

## 7. Formato do arquivo (técnico)

- **CSV UTF-8**, separador **vírgula**, primeira linha é o **cabeçalho** exato acima.
- **Datas** em ISO: `AAAA-MM-DD` (ex.: `2026-06-14`).
- **Booleanos** em minúsculas: `true` / `false`.
- **Números** inteiros, **sem** separador de milhar e **sem** símbolo de moeda.
- Campos de texto podem vir vazios quando "não obrigatório"; `codigo_estoque` **nunca** vazio.
- Um arquivo de vendas + um de estoque por carga (ou um histórico grande na carga inicial).

> Validação dos casos de referência: o negócio já validou manualmente os produtos
> `6797`, `10001` e `10473` no sistema atual — se vierem no extrato, conseguimos conferir a
> paridade dos cálculos antes de virar a chave.

---

## 8. Perguntas para o suporte do One (precisam de resposta)

Estas definições mudam o significado dos dados — melhor alinhar antes da extração:

- **P1 — Vendas: qual data e qual documento?** `qtd_vendida` por dia deve sair de quê:
  **faturamento (NF de venda)**, **pedido**, ou **saída de estoque**? E `dt_ref` é a data de
  **emissão**, de **saída** ou de **competência**? (O legado tratava como "vendas do dia".)
- **P2 — Personalizado:** como o One identifica um item **personalizado**? (tipo de item,
  padrão de SKU, característica, linha de produto?) É o que alimenta `is_personalizado`.
- **P3 — Variação/configuração:** as variações (cor, estampa, tampa…) estão em **um campo
  texto** ou em **características/atributos separados**? Como remontar o `"CHAVE: valor"`?
- **P4 — Código do produto:** qual campo do One é a **chave única** que aparece igual nas
  vendas e no estoque? (precisa ser o **mesmo** `codigo_estoque` nos dois extratos.)
- **P5 — Snapshot completo:** o extrato de estoque consegue trazer **todos** os produtos,
  incluindo **estoque zero** e **descontinuados**? (Se só vier o que tem saldo, quebra a
  classificação de "fora de linha" e "sem estoque".)
- **P6 — Fora de linha:** como o One marca um produto **descontinuado/inativo**? (status do
  cadastro, flag, situação?) É o que alimenta `fora_de_linha`.
- **P7 — Quais depósitos/almoxarifados** compõem o estoque? (só produto acabado? quais
  armazéns entram em `qtd_estoque`/`qtd_reserva`?)
- **P8 — Reserva:** o que conta como **reservado** no One (pedidos confirmados, separação,
  bloqueios)? É o `qtd_reserva`.
- **P9 — Entrega recorrente:** o One consegue **agendar/exportar** esses dois extratos todo dia
  de madrugada (e/ou expor uma API)? Isso define como automatizamos a carga diária.
