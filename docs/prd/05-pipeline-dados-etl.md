# 05 — Pipeline de Dados (ETL e Processamento Diário)

> O coração operacional do sistema: todos os dias, antes do expediente, os dados do ERP/WMS entram no banco e o motor PCP recalcula tudo. Este documento descreve o fluxo atual e o contrato que o novo sistema deve cumprir.

## 1. Fluxo diário (estado atual)

```
03:00–04:00  n8n (Schedule Trigger)
│
├─ Etapa 1: Sincronizar VENDAS do dia anterior
│   ERP → INSERT em vendas_dia (linhas por produto/variação)
│
├─ Etapa 2: Sincronizar ESTOQUE (snapshot do dia)
│   WMS/ERP → DELETE snapshot do dia (se reprocesso) → INSERT estoque_snapshot
│
└─ Etapa 3: Disparar processamento
    POST https://<projeto>.supabase.co/functions/v1/processar-pcp-modular
    Body: { "data_ref": "YYYY-MM-DD" }
```

### 1.1 Edge function `processar-pcp-modular` (orquestrador HTTP)

Sequência interna:

| # | Passo | Crítico? | Falha → |
|---|---|---|---|
| 1 | Validar `data_ref` | sim | aborta |
| 2 | Verificar dados base (conta registros de vendas e snapshot na data) | **não** | só loga warning ⚠️ |
| 3 | `auto_update_fatores_sazonais()` (gatilho mensal) | não | mantém fatores antigos |
| 4 | `orquestrador_pcp_modular(data_ref)` → 4 módulos | sim | resultado parcial |
| 5 | Refresh da materialized view de produtos ativos | não | loga |
| 6 | Consolidar métricas e responder JSON detalhado | — | — |

### 1.2 Orquestrador SQL (4 módulos com isolamento de falha)

Cada módulo roda em bloco try/catch independente; o resultado registra status, tempo (ms) e erro por módulo:

```
1. modulo_classificacao_abcfdn(data)   → classificacao_abc      (doc 02 §2)
2. modulo_parametros_estoque(data)     → estoque_param_v2       (doc 02 §3)
3. modulo_alertas_producao(data)       → alerta_producao        (doc 02 §6)
4. modulo_analise_fora_linha(data)     → analise_fora_linha     (doc 02 §8)
```

Ao final: `datas(dt_ref).status = 'CONCLUIDO'` e resposta com `SUCESSO_COMPLETO` ou `SUCESSO_PARCIAL`.

**Ordem importa:** a classificação precisa rodar antes dos parâmetros (o fator ABC entra na fórmula); parâmetros antes dos alertas (o recomendado define a prioridade).

### 1.3 Timeouts configurados no legado (referência de carga)

| Módulo | Timeout |
|---|---|
| Orquestrador completo | 480 s |
| Classificação | 60 s |
| Parâmetros | 180 s |
| Alertas | 180 s |
| Fora de linha | 120 s |

Tempo real típico de execução completa: 2–5 s para ~2.400 produtos (bem abaixo dos timeouts).

---

## 2. Contrato de dados do ETL (obrigatório no novo sistema)

### 2.1 Vendas (por dia × produto × variação)

| Campo | Tipo | Obrigatório | Validação |
|---|---|---|---|
| `dt_ref` | date | sim | = dia sincronizado |
| `codigo_estoque` | string | sim | não vazio |
| `sku` | string | não | |
| `produto` | string | não | |
| `configuracao` | string | não | padrão `"CHAVE: valor"` |
| `qtd_vendida` | int ≥ 0 | sim | |
| `is_personalizado` | bool | sim | |

### 2.2 Snapshot de estoque (por dia × produto)

| Campo | Tipo | Obrigatório | Validação |
|---|---|---|---|
| `dt_ref` | date | sim | |
| `codigo_estoque` | string | sim | único no dia |
| `sku`, `produto`, `configuracao` | string | não | |
| `qtd_estoque`, `qtd_reserva`, `qtd_disponivel` | int | sim | `disponivel = estoque − reserva` |
| `estoque_min_erp` | int | não | |
| `fora_de_linha` | bool | sim | |

### 2.3 Regras de carga
1. **Idempotência:** recarregar uma data substitui os dados daquela data (delete+insert ou upsert por chave do dia).
2. **Snapshot completo:** o snapshot do dia contém TODOS os produtos (não é incremental).
3. **Vendas append-only**, exceto reprocesso explícito de uma data.
4. O processamento PCP só dispara **após** as duas cargas do dia concluírem.

---

## 3. Requisitos do novo pipeline (correções sobre o legado)

| # | Problema no legado | Requisito novo |
|---|---|---|
| 1 | Processamento roda mesmo **sem dados do dia** (warning ignorado) | **Pré-validação bloqueante**: vendas do dia anterior > 0 e snapshot do dia presente (com tolerância configurável e alerta à equipe) |
| 2 | n8n é ponto único de falha, sem retry documentado | Retries automáticos + alerta (e-mail/WhatsApp/Slack) em falha de qualquer etapa |
| 3 | Horário ambíguo (03:00 vs 04:00 em docs distintos) | Janela definida e monitorada: carga 03:00, processamento 03:30, **dados prontos até 05:00** (SLA) |
| 4 | Logs da execução só na resposta HTTP (efêmeros) | Persistir execução por módulo: início, fim, duração, linhas afetadas, erro |
| 5 | Dois pipelines coexistem (`processar-dados-pcp` v2.5 e `processar-pcp-modular`) | **Um único pipeline** (o modular é o vigente; o monolítico não vai para o novo sistema) |
| 6 | `SUCESSO_PARCIAL` não notifica ninguém | Falha de módulo → notificação + página de status do pipeline |
| 7 | Reprocesso manual exige chamar função na mão | UI/admin: reprocessar data específica e intervalo de datas |

## 4. Health checks (monitoramento contínuo)

| Verificação | Limiar de alerta |
|---|---|
| Snapshot do dia presente até 05:00 | ausente → crítico |
| Variação do nº de produtos no snapshot vs dia anterior | > ±10% → investigar |
| Produtos processados nos parâmetros | queda > 10% vs média 7d |
| Zero alertas gerados por > 7 dias | lógica quebrada |
| CV médio do catálogo | > 0,5 → dados suspeitos |
| Fatores sazonais | desvio real × previsto > 30% no mês corrente → revisar fator |
| Execução do pipeline | duração > 5× média → investigar |

## 5. Cadências

| Processo | Frequência | Gatilho |
|---|---|---|
| Carga de vendas + snapshot | Diária (madrugada) | agendador |
| Motor PCP (4 módulos) | Diária, pós-carga | encadeado |
| Atualização de fatores sazonais | Mensal (auto, >30 dias) | embutido no pipeline |
| Refresh de caches/materializações | Diária, pós-motor | encadeado |
| Expurgo/retenção (novo) | Diária ou semanal | agendador |

## 6. Variáveis de configuração do pipeline (sem valores sensíveis)

| Variável | Uso |
|---|---|
| `ERP_API_URL` (+credenciais) | origem das vendas/estoque |
| URL/credenciais do banco | escrita do ETL |
| Chave de serviço do backend | processamento |
| `OPENAI_API_KEY` | módulos de IA (doc 06) |
| Webhook de notificação | alertas de falha do pipeline |
