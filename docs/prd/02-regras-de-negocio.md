# 02 — Regras de Negócio (Documento Canônico)

> **Fonte da verdade:** funções SQL e views em produção no banco Supabase (`supercopo_pcp`), extraídas em jun/2026. Quando a documentação histórica do projeto diverge do código em produção, **vale o que está aqui**. As divergências conhecidas estão catalogadas no documento 08.

## Convenções e glossário

| Termo | Definição |
|---|---|
| `codigo_estoque` | Identificador único do produto no ERP (chave de negócio em todo o sistema). Texto. |
| `sku` | SKU comercial (pode variar entre LISO e PERSONALIZADO do mesmo código) |
| `configuracao` | Variação do produto: "COR DO PRODUTO: X", "NOME DA ESTAMPA: Y", "COR DA TAMPA: Z", "COR DO DEGRADE: W", "COR DA METALIZACAO: V" |
| `qtd_estoque` | Quantidade física total no snapshot |
| `qtd_reserva` | Quantidade reservada (pedidos) |
| `qtd_disponivel` | `qtd_estoque − qtd_reserva` |
| `fora_de_linha` | Flag booleana vinda do ERP: produto descontinuado |
| `is_personalizado` | Flag em vendas: venda de item personalizado |
| `media_diaria_12m` | Média diária de vendas dos últimos 12 meses, **considerando apenas dias com venda** e **removendo outliers** (ver §3) |
| `cobertura_dias` | Quantos dias o estoque disponível dura na demanda média |
| `data_ref` | Data de referência do processamento (normalmente o dia anterior / dia corrente) |

---

## 1. Consolidação de vendas (pré-requisito de todos os cálculos)

Vendas do mesmo `codigo_estoque` no mesmo dia (ex.: versão LISO + versões PERSONALIZADAS) são **somadas** para fins de análise de demanda.

```
qtd_total(dia, codigo) = SUM(qtd_vendida) GROUP BY dt_ref, codigo_estoque
houve_personalizado    = BOOL_OR(is_personalizado)
```

Para exibição, o nome/SKU "pai" do produto vem do snapshot de estoque mais recente; na ausência, usa-se como fallback o registro de venda priorizando a variação **LISO**.

**Racional:** a demanda de produção é sobre o produto-base; o personalizado consome o mesmo item produzido.

---

## 2. Classificação ABC+F+D+N (vigente — "v3.0")

Executada **diariamente** para a `data_ref`, regravando a classificação do dia (delete + insert por `dt_calculo`). Cada produto recebe exatamente **uma classe**, avaliada nesta **ordem de precedência**:

### 2.1 Classe F — Fora de linha (precedência 1)
- **Critério:** `fora_de_linha = true` no snapshot de estoque mais recente.
- **Fator de estoque:** `0.10` (−90%).

### 2.2 Classe D — Sem vendas há 6 meses (precedência 2)
- **Critério:** produto ativo (`fora_de_linha = false`) **sem nenhuma venda nos últimos 180 dias**.
- **Fator de estoque:** `0.30` (−70%).

### 2.3 Classe N — Produto novo (precedência 3)
- **Critério:** primeira venda do produto (considerando **todo o histórico** de vendas, sem janela) ocorreu há **menos de 60 dias** da `data_ref`, e o produto está ativo.
- **Fator de estoque:** `0.80` (−20%).
- **Racional:** produtos em rampagem não têm histórico estável; não devem ser classificados como C por volume baixo.

### 2.4 Classes A/B/C — Curva de Pareto (produtos ativos maduros)
- **Universo:** produtos ativos, com vendas no período, **excluindo** os já classificados como F/D/N.
- **Janela de volume:** **540 dias (18 meses)** de vendas somadas (`volume_18m`). A janela foi ampliada de 12 para 18 meses para neutralizar sazonalidade.
- **Cálculo:**

```
percentual_acumulado = (volume_acumulado_em_ordem_decrescente / volume_total) × 100

Classe A: percentual_acumulado ≤ 80   → fator_estoque = 1.20 (+20%)
Classe B: percentual_acumulado ≤ 95   → fator_estoque = 1.00
Classe C: percentual_acumulado > 95   → fator_estoque = 0.80 (−20%)
```

### 2.5 Resumo das classes

| Classe | Significado | Critério | Fator estoque | Meta de cobertura (dias) |
|---|---|---|---|---|
| **A** | Estratégico (~80% do volume) | Pareto ≤ 80% | 1.20 | **45** |
| **B** | Importante (~15% do volume) | Pareto ≤ 95% | 1.00 | **30** |
| **C** | Complementar (~5% do volume) | Pareto > 95% | 0.80 | **15** |
| **D** | Baixíssima rotação | 6 meses sem vendas | 0.30 | **10** |
| **F** | Fora de linha | Flag do ERP | 0.10 | **5** |
| **N** | Novo (rampagem) | 1ª venda < 60 dias | 0.80 | **20** |
| (sem classe) | Default defensivo | — | 1.00 | 15 |

- A classificação é **persistida com data** (`dt_calculo`), formando histórico diário.
- Consumidores sempre usam a classificação **mais recente** (`MAX(dt_calculo)` por produto).
- Produto sem classificação é tratado como **C** nas views (default defensivo).

Distribuição real observada (jun/2026): A=165, B=346, C=671, D=1.012, F=177, N=9.

---

## 3. Parâmetros de estoque por produto (recalculados diariamente)

### 3.1 Janela e base estatística
- **Janela:** vendas dos últimos **12 meses** até a `data_ref`, considerando apenas registros com `qtd_vendida > 0`.
- A média é calculada **sobre os dias com venda** (não sobre 365 dias corridos).

### 3.2 Remoção de outliers (método IQR)
```
Q1, Q3 = percentis 25 e 75 da distribuição diária de qtd_vendida (12m)
limite_superior = Q3 + 1.5 × (Q3 − Q1)

media_sem_outliers  = AVG(qtd_vendida | qtd_vendida ≤ limite_superior)
desvio_sem_outliers = STDDEV(qtd_vendida | qtd_vendida ≤ limite_superior)
outliers_detectados = COUNT(qtd_vendida > limite_superior)
```
> Apenas o limite **superior** é aplicado (picos de pedidos grandes); não se removem vendas baixas.

### 3.3 Coeficiente de variação
```
coef_variacao = desvio_sem_outliers / media_sem_outliers   (0 se média = 0)
```
Usado como indicador de previsibilidade da demanda (CV > 0,5 = demanda errática).

### 3.4 Produtos com histórico insuficiente (`dias_com_vendas < 10`)
Valores **default fixos** (não calculados):

| Parâmetro | Valor default |
|---|---|
| `media_diaria_12m` | 50.0 |
| `estoque_min_15d` | 750 |
| `estoque_seguranca` | 250 |
| `estoque_total_recomendado` | máx. 1.000 |

> ⚠️ Este default é reconhecidamente grosseiro (ver doc 08). O novo projeto deve tratá-lo como parâmetro configurável e sinalizar o produto como `SEM_HISTORICO_CONFIAVEL`.

### 3.5 Fórmulas principais (produto com histórico ≥ 10 dias de venda)

```
fator_sazonal = fator do mês da data_ref (ver §4)
fator_abc     = fator_estoque da classe vigente (§2.5)

estoque_min_15d           = CEIL(media_sem_outliers × 15 × fator_sazonal × fator_abc)
estoque_seguranca         = CEIL(desvio_sem_outliers × 1.28 × fator_sazonal)
estoque_total_recomendado = MIN( estoque_min_15d + estoque_seguranca,
                                 CEIL(media_sem_outliers × 60) )
```

- **1.28** = z-score para 90% de nível de serviço.
- O teto de **60 dias de cobertura** impede recomendações infladas pela sazonalidade/segurança.
- Resultado gravado por produto com `dt_calc` (upsert — mantém apenas o cálculo mais recente por produto).

### 3.6 Recalibração por meta de cobertura ABC (regra complementar)

Existe uma função de recalibração que substitui o estoque recomendado pela **meta de dias da classe**:

```
dias_meta = {A: 45, B: 30, C: 15, D: 10, F: 5, N: 20, default: 15}

estoque_total_recomendado = ROUND(media_diaria × dias_meta × fator_sazonal) + estoque_seguranca
estoque_min_15d           = ROUND( ROUND(media_diaria × dias_meta × fator_sazonal) × 0.70 )   # 70% do alvo
```

> 📌 **Decisão para o novo projeto:** estas duas formulações (§3.5 base estatística de 15 dias e §3.6 meta ABC) coexistem no legado — a §3.5 roda no pipeline diário e a §3.6 foi aplicada em lote como correção. O novo motor deve **unificar em §3.6 (meta por classe ABC)** como fórmula principal, mantendo o estoque de segurança estatístico da §3.5 e o teto de 60 dias. Essa é a direção documentada e validada pelo negócio (`IMPLEMENTACAO_NOVAS_REGRAS_ESTOQUE_ABC`).

---

## 4. Sazonalidade dinâmica

### 4.1 Modelo
Um **fator multiplicador por mês do ano** (1–12), persistido em tabela própria:

```
fator_mes = media_diaria_vendas(mes, ano_anterior) / media_diaria_vendas(ano_anterior inteiro)
fator_mes = CLAMP(fator_mes, 0.5, 2.0)        # suavização: nunca menor que 0,5x nem maior que 2x
```

### 4.2 Atualização automática
- A cada execução do pipeline diário, verifica-se a data da última atualização dos fatores.
- **Gatilho:** recalcular se o mês da última atualização ≠ mês corrente **ou** se passaram **> 30 dias**.
- A atualização é **failsafe**: erro no recálculo não interrompe o pipeline (mantém fatores anteriores).
- Toda atualização gera log em tabela de logs do sistema.

### 4.3 Aplicação
O fator do **mês corrente** multiplica `estoque_min_15d`, `estoque_seguranca` e as recomendações de produção (§3 e §7).

### 4.4 Valores em produção (calculados sobre 2025, vigentes em jun/2026 — referência)

| Mês | Fator | | Mês | Fator |
|---|---|---|---|---|
| Jan | 1.25 | | Jul | 0.64 |
| Fev | 0.99 | | Ago | 0.63 |
| Mar | 1.00 | | Set | 0.71 |
| Abr | 0.87 | | Out | 0.90 |
| Mai | 0.81 | | Nov | 0.67 |
| Jun | 0.62 | | **Dez** | **2.00** (atinge o teto) |

### 4.5 Monitoramento
View de monitoramento compara, para o mês corrente: média diária **real** vs **prevista** (média do ano anterior × fator), expondo o `desvio_percentual_real_vs_previsto`. Serve para auditar a qualidade do fator.

---

## 5. Cobertura e status de estoque

### 5.1 Cobertura em dias
```
cobertura_dias = qtd_disponivel / media_diaria_12m      (arredondado a 1 casa)
cobertura_dias = 999                                     se media_diaria_12m = 0 ou nula (sentinela "sem histórico")
```

### 5.2 Status do produto (regra canônica — view principal `v_produtos_ativos`)

Avaliado nesta ordem (primeira condição verdadeira vence):

| Ordem | Status | Condição |
|---|---|---|
| 1 | `SEM_ESTOQUE` | `qtd_disponivel ≤ 0` |
| 2 | `FORA_DE_LINHA` | `fora_de_linha = true` |
| 3 | `SEM_HISTORICO` | `media_diaria_12m = 0` ou nula |
| 4 | `CRITICO` | classe **A** e `cobertura_dias ≤ 15` |
| 5 | `CRITICO` | classe **B** e `cobertura_dias ≤ 10` |
| 6 | `CRITICO` | classe **C** (ou sem classe) e `cobertura_dias ≤ 5` |
| 7 | `ESTOQUE_BAIXO` | `qtd_disponivel < estoque_min_15d` |
| 8 | `BAIXO` | `qtd_disponivel < estoque_seguranca` |
| 9 | `ADEQUADO` | `qtd_disponivel ≤ estoque_total_recomendado` |
| 10 | `ALTO` | `qtd_disponivel ≤ estoque_total_recomendado × 1.5` |
| 11 | `EXCESSIVO` | acima disso |

**Limiares de criticidade por classe (memorizar):** A ≤ 15 dias · B ≤ 10 dias · C ≤ 5 dias.

### 5.3 Faixas de cobertura "adequada" por classe (avaliação da recomendação)

Usadas para auditar se o estoque recomendado está na faixa-alvo da classe:

| Classe | Faixa adequada | Alvo | Limite crítico |
|---|---|---|---|
| A | 31–60 dias | 45d | ≤ 15 dias |
| B | 26–55 dias | 40d (regra antiga) / 30d (meta vigente) | ≤ 10 dias |
| C | 13–20 dias | 16d (regra antiga) / 15d (meta vigente) | ≤ 5 dias |

> O novo projeto deve adotar **um único conjunto**: meta = {45, 30, 15} com faixa adequada = meta ± margem definida em configuração (sugestão: −30%/+35%).

### 5.4 Percentual da meta (UI — coluna "Recomendada")
```
percentual = (qtd_disponivel / estoque_total_recomendado) × 100
```

| Faixa | Interpretação |
|---|---|
| ≥ 150% | Estoque alto |
| 100–149% | Adequado |
| 80–99% | Planejar reposição |
| 50–79% | Repor em breve |
| 25–49% | Urgente |
| 0–24% | Crítico / sem estoque |

---

## 6. Alertas de produção (gerados diariamente)

### 6.1 Universo
Produtos do snapshot mais recente que:
- **não** estão fora de linha; e
- têm `media_diaria_12m > 0` (histórico válido).

Os alertas do dia são **regravados** a cada processamento (delete da `data_ref` + insert).

### 6.2 Prioridade base (percentual do estoque recomendado)

```
CRITICO: qtd_disponivel ≤ 0  OU  qtd_disponivel < 20% do estoque_total_recomendado
ALTO:    qtd_disponivel < 50% do estoque_total_recomendado
MEDIO:   qtd_disponivel < 80% do estoque_total_recomendado
(sem alerta acima de 80%)
```

### 6.3 Elevação por classe A
Produtos **classe A** sobem um nível de prioridade:

```
ALTO  + classe A → CRITICO
MEDIO + classe A → ALTO
```

### 6.4 Conteúdo do alerta
Cada alerta carrega: data, código, SKU, produto, **prioridade**, `qtd_sugerida` (§7.1) e `cobertura_dias` no momento da geração.

> ⚠️ No legado, a prioridade é gravada no campo `configuracao` da tabela de alertas (reuso indevido de coluna). No novo modelo, criar campo próprio `prioridade`.

### 6.5 Ordenação para a fila de produção
1. Prioridade (CRÍTICO → ALTO → MÉDIO);
2. Classe ABC (A primeiro);
3. `qtd_sugerida` decrescente.

---

## 7. Recomendação de produção/reposição

### 7.1 Quantidade sugerida (regra do banco — usada em alertas e tabela de estoque)
```
qtd_sugerida = MAX(0, estoque_total_recomendado − qtd_disponivel)
qtd_sugerida = 0   se fora_de_linha = true ou media_diaria_12m = 0
```

### 7.2 Solicitação de produção inteligente (regra do Centro de Comando — UI/IA)

Cálculo mais rico usado ao gerar uma solicitação de produção a partir do detalhe do produto:

```
meta_dias        = {A: 45, B: 30, C: 15, D: 10, F: 5, N: 20}[classe]
estoque_ideal    = ROUND(media_diaria_12m × meta_dias)
qtd_necessaria   = MAX(0, estoque_ideal − qtd_estoque + estoque_seguranca)

fator_urgencia   = 1.5  se cobertura_dias < 7
                   1.2  se cobertura_dias < 15
                   1.0  caso contrário

qtd_final = ROUND(qtd_necessaria × fator_urgencia × fator_sazonal)

# Proteção contra ruptura iminente:
se cobertura_dias < 3:
    qtd_final = MAX(qtd_final, ROUND(media_diaria_12m × 15))
```

**Prioridade da solicitação:**

| Prioridade | Condição | Lead time |
|---|---|---|
| Alta | cobertura < 7 dias OU alerta crítico ativo | 7 dias |
| Média | cobertura < 15 dias OU classe A | 10 dias |
| Baixa | demais | 15 dias |

**Aprovação automática** (conceito a implementar de verdade no novo projeto): `qtd_final < 1.000` e prioridade ≠ alta.

### 7.3 Escalonamento por criticidade (engine de insights)

Variante usada nos insights da página de produto (mesma meta, escalonamento de quantidade):

```
se cobertura < meta × 0.3:  produzir IMEDIATO,    qtd = qtd_necessaria × 1.2,  prioridade alta
se cobertura < meta × 0.6:  produzir em 1 semana, qtd = qtd_necessaria × 0.8,  prioridade média
se cobertura < meta × 1.0:  produzir em 2 semanas,qtd = qtd_necessaria × 0.6,  prioridade média
se cobertura > meta × 1.5:  AGUARDAR (estoque excessivo)
senão:                      MONITORAR
```

Ajuste por tendência de demanda: se a variação prevista de demanda > |20%|, multiplicar a quantidade por 1.1 (alta) ou 0.9 (queda).

> 📌 **Decisão para o novo projeto:** unificar §7.1/§7.2/§7.3 em **um único serviço de recomendação** com a meta ABC como base, fator de urgência e fator sazonal como multiplicadores e o escalonamento por criticidade como política de timing. Hoje são três implementações paralelas.

---

## 8. Análise de fora de linha (ciclo de vida do produto)

Executada diariamente. Gera **sugestões** (não aplica automaticamente, exceto política opcional).

### 8.1 Sistema de pontuação (0–20 pontos)

| Critério | Condição | Pontos |
|---|---|---|
| Vendas 12m | = 0 | **8** |
| | ≤ 5 unidades | 6 |
| | ≤ 10 unidades | 4 |
| Volume 12m | = 0 | **6** |
| | ≤ 50 | 4 |
| | ≤ 100 | 2 |
| Classe | C | **4** |
| | B | 2 |
| Recência | sem venda ≥ 365 dias (ou nunca vendeu) | **6** |
| | sem venda ≥ 180 dias | 4 |
| | sem venda ≥ 90 dias | 2 |

(Critérios atingidos são gravados junto com a sugestão, ex.: `SEM_VENDAS_12M`, `CLASSE_C`, `SEM_VENDA_1_ANO`.)

### 8.2 Decisões

| Situação | Regra |
|---|---|
| **Sugerir SAIR de linha** | produto ATIVO com pontuação **≥ 8** |
| **Sugerir VOLTAR à linha** | produto FORA DE LINHA com pontuação **≤ 4**, vendas 12m > 0 e última venda ≤ 90 dias |

### 8.3 Níveis de certeza e workflow

| Pontuação | Nível | Ação |
|---|---|---|
| ≥ 15 | ALTA_CERTEZA | Pronta para aplicação (pode ser automática, mediante política) |
| 10–14 | MEDIA_CERTEZA | Aguardando análise humana |
| < 10 | BAIXA_CERTEZA | Apenas monitoramento |

A aplicação registra: data, usuário (`aplicado_por`), observações. **Requisito novo:** a aplicação deve refletir no ERP (hoje é só registro local).

---

## 9. Indicadores do dashboard (regras de agregação)

| Indicador | Regra |
|---|---|
| Total de produtos | COUNT no snapshot mais recente |
| Produtos ativos / % | `fora_de_linha = false` |
| Sem estoque | `qtd_disponivel ≤ 0` |
| Críticos | status `CRITICO` (§5.2), com breakdown por classe A/B/C |
| Cobertura média | AVG(`cobertura_dias`) excluindo sentinela 999 e sem estoque |
| Cobertura média por classe | mesma média, agrupada por classe ABC |
| Distribuição de cobertura | contagem por status (§5.2) + percentuais |
| Total a produzir | SUM(`qtd_sugerida`) |
| Estoque total (unidades) | SUM(`qtd_estoque`) |
| Distribuição ABC (produtos e estoque físico) | contagem e SUM(`qtd_estoque`) por classe |
| Histórico de estoque 30 dias | SUM(`qtd_estoque`) por dia × classe (gráfico de tendência) |

### 9.1 Metas de distribuição física de estoque por classe (painel de metas)

| Classe | Meta de participação no estoque físico |
|---|---|
| A | **50%** |
| B | **30%** |
| C | **20%** |
| D | **0%** (idealmente zero capital em D) |

Status "meta atingida": diferença absoluta ≤ 3 pontos percentuais.

### 9.2 Thresholds visuais dos cards (alerta de cor)

| Card | Vermelho | Amarelo |
|---|---|---|
| % críticos | > 20% | > 10% |
| Sem estoque | > 100 produtos | — |
| Cobertura média | < 30 dias | < 60 dias |
| Reposição total | > 100 mil un | — |
| Excessivos | > 500 produtos | — |

---

## 10. Nomenclatura e formatação obrigatórias

1. **"Produzir" / "Produção"** — nunca "Comprar" (90% produção própria).
2. Coluna de alvo de estoque chama-se **"Recomendada"** (não "Qtd Mínima") e exibe `estoque_total_recomendado`.
3. A variação do produto (cor/estampa) é exibida junto ao nome: `"{produto} - {valor da configuracao}"`, extraindo o que vem após `":"` no campo `configuracao`.
4. Cobertura `999` é exibida como **"Sem histórico"**, nunca como número.
5. Datas no formato brasileiro; quantidades com separador de milhar.

---

## 11. Tabela-resumo das constantes de negócio (para arquivo de configuração)

```yaml
classificacao:
  janela_abc_dias: 540          # 18 meses
  janela_classe_d_dias: 180     # 6 meses sem vendas → D
  janela_produto_novo_dias: 60  # 1ª venda < 60 dias → N
  pareto_a: 80                  # % acumulado
  pareto_b: 95
  fator_estoque: {A: 1.20, B: 1.00, C: 0.80, D: 0.30, F: 0.10, N: 0.80}

metas_cobertura_dias: {A: 45, B: 30, C: 15, D: 10, F: 5, N: 20, default: 15}

limiar_critico_dias: {A: 15, B: 10, C: 5}

parametros_estoque:
  janela_vendas_meses: 12
  min_dias_com_vendas: 10
  outlier_iqr_mult: 1.5
  z_score_seguranca: 1.28       # 90% nível de serviço
  dias_base_minimo: 15
  teto_cobertura_dias: 60
  defaults_sem_historico: {media: 50, min: 750, seguranca: 250, recomendado_max: 1000}

sazonalidade:
  clamp_min: 0.5
  clamp_max: 2.0
  atualizar_apos_dias: 30

alertas:
  critico_pct: 0.20             # < 20% do recomendado
  alto_pct: 0.50
  medio_pct: 0.80
  elevar_classe_a: true

reposicao:
  fator_urgencia: {cobertura_lt_7: 1.5, cobertura_lt_15: 1.2, default: 1.0}
  protecao_ruptura_dias: 3      # se cobertura < 3 → qtd >= media × 15
  aprovacao_automatica: {qtd_max: 1000, exceto_prioridade: alta}
  lead_time_dias: {alta: 7, media: 10, baixa: 15}

fora_de_linha:
  limiar_sugerir_saida: 8
  limiar_sugerir_volta: 4
  alta_certeza: 15
  media_certeza: 10

metas_estoque_fisico_pct: {A: 50, B: 30, C: 20, D: 0}
```

> **Requisito do novo projeto:** todas estas constantes devem viver em **configuração editável** (tela de Configurações + auditoria de mudanças), não hardcoded como no legado.
