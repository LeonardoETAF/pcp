# 04 — Modelo de Dados

> Levantado **diretamente do banco em produção** (Supabase, schema `supercopo_pcp`) em jun/2026. Serve como referência do domínio para o novo modelo — **não** como DDL a copiar. A seção 5 indica o que reaproveitar e o que redesenhar.

## 1. Visão geral

```
ENTRADA (ETL diário)            DERIVADAS (motor PCP diário)        APOIO
├── vendas_dia                  ├── classificacao_abc               ├── fatores_sazonais
└── estoque_snapshot            ├── estoque_param_v2                ├── datas (controle)
                                ├── alerta_producao                 ├── logs_sistema
                                ├── analise_fora_linha              ├── primeira_venda_produto
                                └── sugestoes_fora_linha            └── chat_conversations/messages
```

Volumetria atual (jun/2026):

| Tabela | Linhas | Período | Observação |
|---|---|---|---|
| `vendas_dia` | 106.488 | 02/01/2024 → hoje | 2.582 produtos distintos |
| `estoque_snapshot` | 871.609 | 03/06/2025 → hoje | snapshot diário completo (~2.400/dia), 2.838 produtos |
| `classificacao_abc` | 869.324 | diário desde 05/06/2025 | histórico completo de classificações |
| `alerta_producao` | 52.776 | diário | ~150–400 alertas/dia, 1.003 produtos já alertados |
| `estoque_param_v2` | 2.346 | só estado atual (upsert) | 1 linha por produto |
| `analise_fora_linha` | **33.767.350** | ⚠️ | **sem expurgo — bug de retenção** |
| `fatores_sazonais` | 12 | 1 por mês | |
| `datas` | 104 | | controle de processamento |

---

## 2. Tabelas de entrada (alimentadas pelo ETL)

### 2.1 `vendas_dia` — vendas diárias por produto

| Coluna | Tipo | Regra |
|---|---|---|
| `id` | bigserial | PK técnica |
| `dt_ref` | date NOT NULL | dia da venda |
| `codigo_estoque` | text NOT NULL | chave de negócio |
| `sku` | text | pode diferir entre variações |
| `produto` | text | nome |
| `configuracao` | text | variação (cor/estampa) |
| `qtd_vendida` | integer | unidades |
| `is_personalizado` | boolean | LISO × PERSONALIZADO |

- **Granularidade:** pode haver **múltiplas linhas por (dt_ref, codigo_estoque)** (variações LISO/PERSONALIZADO) — a consolidação é feita na leitura (doc 02 §1).
- Histórico completo preservado (base de todos os cálculos).

### 2.2 `estoque_snapshot` — foto diária do estoque

| Coluna | Tipo | Regra |
|---|---|---|
| `dt_ref` | date NOT NULL | dia do snapshot |
| `codigo_estoque` | text NOT NULL | |
| `sku`, `produto`, `configuracao` | text | dados cadastrais do dia |
| `qtd_estoque` | integer | físico total |
| `qtd_reserva` | integer | reservado |
| `qtd_disponivel` | integer | estoque − reserva |
| `estoque_min_erp` | integer | mínimo cadastrado no ERP (referência, não usado nos cálculos) |
| `fora_de_linha` | boolean | flag de descontinuação do ERP |

- PK lógica: `(dt_ref, codigo_estoque)`.
- Todo o sistema usa o **snapshot mais recente** (`MAX(dt_ref)`); o histórico alimenta gráficos de tendência.

---

## 3. Tabelas derivadas (escritas pelo motor PCP)

### 3.1 `classificacao_abc` — classificação diária

| Coluna | Tipo | Regra |
|---|---|---|
| `codigo_estoque` | text NOT NULL | |
| `classe_abc` | char(1) NOT NULL | A/B/C/D/F/N |
| `volume_12m` | bigint | **atenção: apesar do nome, contém o volume da janela ABC (18 meses)** |
| `percentual_acumulado` | numeric | posição na curva de Pareto |
| `fator_estoque` | numeric (default 1.00) | multiplicador da classe |
| `dt_calculo` | date NOT NULL | data do cálculo |

- Regravada por dia (`DELETE WHERE dt_calculo = data` + INSERT).
- Consumo: sempre `MAX(dt_calculo)` por produto.
- **Novo modelo:** renomear `volume_12m` → `volume_janela` (ou registrar a janela em coluna própria); considerar particionamento ou retenção (histórico diário completo cresce ~2.400 linhas/dia).

### 3.2 `estoque_param_v2` — parâmetros estatísticos (estado atual)

| Coluna | Tipo | Conteúdo |
|---|---|---|
| `codigo_estoque` | text PK | |
| `media_diaria_12m` | numeric | média sem outliers (dias com venda) |
| `desvio_sem_outliers` | numeric | |
| `coef_variacao` | numeric | desvio/média |
| `fator_sazonal_atual` | numeric | fator do mês aplicado no cálculo |
| `mes_referencia` | integer | mês do fator |
| `estoque_min_15d` | integer | doc 02 §3.5 |
| `estoque_seguranca` | integer | doc 02 §3.5 |
| `estoque_total_recomendado` | integer | doc 02 §3.5/§3.6 |
| `dias_com_vendas` | integer | qualidade do histórico |
| `outliers_detectados` | integer | |
| `ultima_venda` | date | |
| `dt_calc` | date | data do último cálculo |
| `created_at`, `updated_at` | timestamp | |

- UPSERT por produto (sem histórico). **Novo modelo:** avaliar guardar histórico (ex.: tabela de séries) para auditar evolução dos parâmetros.

### 3.3 `alerta_producao` — alertas diários

| Coluna | Tipo | Conteúdo |
|---|---|---|
| `dt_alerta` | date NOT NULL | |
| `codigo_estoque` | text NOT NULL | |
| `sku`, `produto` | text | |
| `configuracao` | text | ⚠️ **no legado armazena a PRIORIDADE** (CRITICO/ALTO/MEDIO) — reuso indevido da coluna |
| `qtd_sugerida` | integer | doc 02 §7.1 |
| `cobertura_dias` | numeric | cobertura no momento da geração |

- **Novo modelo:** coluna `prioridade` própria; manter `configuracao` para a variação real do produto.

### 3.4 `analise_fora_linha` + `sugestoes_fora_linha` — ciclo de vida

`analise_fora_linha` (análise diária detalhada):
`codigo_estoque`, `nome_produto`, `acao_sugerida` (PARA_FORA_LINHA / PARA_VOLTA_LINHA), `pontuacao_total` (0–20), `criterios_atingidos[]`, `vendas_12m`, `volume_12m`, `dias_sem_venda`, `valor_estoque` (⚠️ sempre 0 no legado — sem preço), `classe_abc`, `status_atual`, `data_analise`, `aplicado`, `data_aplicacao`, `aplicado_por`, `observacoes`.

`sugestoes_fora_linha` (fila de aprovação):
`codigo_estoque`, `status_atual`/`status_sugerido` (boolean fora_de_linha), `motivo`, `criterios_atendidos` (jsonb), `pontuacao_risco`, `data_analise`, `data_aplicacao`, `aplicado_por`, `observacoes`.

- ⚠️ **33,7 milhões de linhas acumuladas** em `analise_fora_linha` por regravação diária sem expurgo. **Novo modelo:** unificar as duas tabelas em uma só entidade "sugestão de ciclo de vida" com estados (`gerada → em_analise → aplicada/recusada/expirada`) e retenção definida (ex.: 90 dias para não aplicadas).

---

## 4. Tabelas de apoio

### 4.1 `fatores_sazonais`
`mes` (1–12, PK), `fator_multiplicador` (numeric, clamp 0.5–2.0), `descricao` (origem do fator), `ativo`, `created_at`, `updated_at`.

### 4.2 `datas` — controle do pipeline
`dt_ref` (PK), `status` (PENDENTE → CONCLUIDO / ERRO_PARCIAL). **Novo modelo:** evoluir para tabela de execuções com resultado por módulo, duração e erros (observabilidade).

### 4.3 `logs_sistema`
`modulo`, `acao`, `detalhes`, `timestamp_log`. Usada pelo auto-update de sazonalidade. **Novo modelo:** logging estruturado.

### 4.4 `primeira_venda_produto`
Cache de `MIN(dt_ref)` de vendas por produto (suporte à classe N). Vazia no momento (a função v3.0 calcula inline). Opcional no novo modelo (índice/materialização de performance).

### 4.5 `chat_conversations` / `chat_messages`
Histórico do Chat IA (com RLS por `auth.uid()` — únicas tabelas com RLS no legado):
- Conversa: `id` uuid, `title`, `summary`, `tags[]`, `is_favorite`, `total_messages`, `last_message_at`, `user_id`, `metadata` jsonb.
- Mensagem: `id`, `conversation_id`, `role` (user/assistant), `content`, `function_called`, `function_args` jsonb, `data_used` jsonb, `tokens_used`, `response_time_ms`, `user_id`.
- Triggers mantêm contadores da conversa; funções geram título e tags automáticas.

---

## 5. Views (camada de leitura — regras consolidadas)

| View | Papel | Regras embutidas |
|---|---|---|
| `v_produtos_ativos` | **View principal** — junta snapshot atual + parâmetros + classificação mais recente | `cobertura_dias`, `status_estoque` (doc 02 §5.2), `qtd_sugerida` (doc 02 §7.1), flags `tem_parametros/tem_classificacao/tem_estoque`, defaults defensivos (classe C, fator 1.0) |
| `mv_produtos_ativos` | Materialização da anterior (refresh pós-processamento) | cache de performance |
| `v_alertas_completos` | Alertas do dia enriquecidos com dados do produto | extração da cor a partir de `configuracao` (prefixos COR DO PRODUTO:/NOME DA ESTAMPA:/COR DA TAMPA:/COR DO DEGRADE:/COR DA METALIZACAO:/genérico `:`), nome exibido = `produto - cor` |
| `v_cobertura_v2` | Legado — status por faixas fixas 10/15/25/30 dias | ⚠️ divergente da regra canônica — **não levar** |
| `v_regras_estoque_abc` | Auditoria: meta de dias da classe vs cobertura recomendada efetiva | `dias_regra_classe` {45,30,15,10,5,20}, `status_regra` = REGRA_APLICADA se |diferença| ≤ 2 dias |
| `v_regras_estoque` | Variante com faixas A 31–60/B 26–55/C 13–20 e justificativas textuais | textos das regras por classe |
| `v_dashboard_fora_linha` | Fila de sugestões do dia com `nivel_certeza` (≥15 ALTA / ≥10 MEDIA / BAIXA) e `status_sugestao` (APLICADA / PRONTA_APLICACAO / AGUARDANDO_ANALISE) | doc 02 §8.3 |
| `vw_vendas_consolidadas` | Consolidação LISO+PERSONALIZADO (doc 02 §1) | prioriza SKU "LISO" como pai |
| `vw_fatores_sazonais_monitor` | Auditoria mensal: fator vigente vs realidade | `desvio_percentual_real_vs_previsto` |
| `chat_conversation_stats` | Métricas por conversa do chat | agregações |

## 6. Funções/RPCs do banco (inventário funcional)

### 6.1 Motor de processamento (núcleo a reimplementar)

| Função | Papel (regra no doc 02) |
|---|---|
| `orquestrador_pcp_modular(data)` | Executa os 4 módulos em sequência com isolamento de falha e telemetria por módulo; marca a data como CONCLUIDO |
| `modulo_classificacao_abcfdn(data)` | Classificação 6 níveis (§2) |
| `modulo_parametros_estoque(data)` | Parâmetros estatísticos (§3) |
| `modulo_alertas_producao(data)` | Alertas (§6) |
| `modulo_analise_fora_linha(data)` | Ciclo de vida (§8) |
| `auto_update_fatores_sazonais()` | Gatilho mensal de sazonalidade (§4) |
| `calcular_fatores_sazonais_dinamicos(data, anos)` | Recálculo dos 12 fatores |
| `obter_fator_sazonal(mes)` | Lookup com fallback 1.0 |
| `calcular_estoque_recomendado_abc(media, classe, seguranca, sazonal)` | Fórmula meta ABC (§3.6) |
| `aplicar_sugestoes_fora_linha*` | Aplicação de sugestões com política de pontuação mínima |

### 6.2 API de leitura usada pelo frontend (~18 RPCs)

Resumos: `get_resumo_estoque_geral`, `get_resumo_estoque_rapido`, `get_dashboard_completo`, `get_dashboard_metrics`, `get_produtos_detalhados`.
Alertas: `get_alertas_completos`, `get_alertas_criticos_detalhados`.
Cobertura: `get_distribuicao_cobertura_estoque`, `get_cobertura_media_abc`.
ABC: `get_distribuicao_abc_estoque`, `get_distribuicao_fisica_abc`, `get_produtos_abc_completo`, `get_top_produtos_abc_otimizado(limite)`, `get_estoque_historico_abc(dias)`.
Estoque: `get_produtos_ativos_paginado(offset, limit)`.
Chat: `search_conversations(...)`, `generate_conversation_title`, `generate_automatic_tags`.

> **Novo projeto:** estes RPCs definem o **contrato da API de leitura** (payloads e agregações). A implementação pode ser REST/GraphQL/RPC — o importante é cobrir as mesmas agregações.

---

## 7. O que reaproveitar × o que reconstruir

### ✅ Reaproveitar (conceito e regras)
1. **Modelo de entrada** `vendas_dia` + `estoque_snapshot` — simples, provado, casa com o ETL.
2. **Separação entrada → derivadas → views de leitura**.
3. Regras das views `v_produtos_ativos`, `v_alertas_completos`, `v_regras_estoque_abc` (regra canônica).
4. Histórico diário de classificação (permite análises de migração de classe).
5. Estrutura do chat (conversas/mensagens/métricas) com RLS.
6. **Dados históricos**: migrar `vendas_dia` completa (2024→) e snapshots; são o combustível dos cálculos. Demais tabelas derivadas podem ser **recalculadas** no novo sistema.

### 🔄 Reconstruir/Redesenhar
1. `alerta_producao`: coluna `prioridade` própria (+id, +status de tratamento do alerta).
2. Ciclo de vida: unificar `analise_fora_linha` + `sugestoes_fora_linha` com máquina de estados e **retenção** (evitar os 33,7M de linhas).
3. `estoque_param_v2`: decidir entre estado atual + histórico de auditoria.
4. `datas` → tabela de execuções do pipeline (por módulo, com duração/erro).
5. Eliminar views/funções duplicadas (`v_cobertura_v2`, `get_dashboard_metrics` vs `get_dashboard_completo`, duas versões de `aplicar_sugestoes_fora_linha`, `processar_dia` vs `_v2` etc.) — uma só versão de cada conceito.
6. Adicionar **dimensão financeira**: preço/custo unitário por produto (hoje inexistente — `valor_estoque` sempre 0) para ROI, capital parado e priorização por valor.
7. **RLS/autorização em todas as tabelas** desde o dia 1 (no legado, só o chat tem RLS).
8. Nomes de colunas honestos (`volume_12m` que contém 18m, `estoque_min_15d` que pode não ser 15 dias após recalibração ABC).

### 🗑️ Não levar
- `log_views_removidas`, views legadas já divergentes, funções obsoletas (`calcular_abc` com fatores 1.3/1.1/0.9, `processar_dia` v1, `modulo_classificacao_abc` v1, `modulo_alertas_producao_v2` não usada).
- Tabelas de outros sistemas que coabitam o mesmo banco (o projeto atual divide o Supabase com CRM, SAC, logística etc. — ver doc 07 §4).
