# 01 — Visão Geral do Produto

## 1. O que é o sistema

O **SuperCopo PCP** é a plataforma de Planejamento e Controle de Produção da SuperCopo. Ele responde diariamente, de forma automática, às perguntas centrais da operação:

1. **O que produzir hoje?** (alertas de produção priorizados)
2. **Quanto produzir de cada item?** (quantidade sugerida com base em meta de cobertura por classe)
3. **Quais produtos merecem mais ou menos estoque?** (classificação ABC+F+D+N)
4. **Quais produtos devem sair de linha ou voltar?** (análise de ciclo de vida com pontuação de risco)
5. **Como a sazonalidade afeta a demanda?** (fatores mensais dinâmicos)

## 2. Contexto operacional

| Aspecto | Descrição |
|---|---|
| Negócio | Fábrica de copos e produtos plásticos, com forte personalização (estampas, cores, degradês, metalização) |
| Modo de suprimento | **~90% produção própria** — a linguagem do sistema é "Produzir", não "Comprar" |
| Lead time de produção | 7 dias (urgente) a 15 dias (normal) |
| Catálogo | ~2.800 códigos de estoque ativos no snapshot; um mesmo produto pode ter variações LISO e PERSONALIZADO que se consolidam sob o mesmo `codigo_estoque` |
| Sazonalidade | Muito forte: dezembro chega a **2x** a média anual; junho-agosto caem para **~0,62x** |
| Fonte de dados | ERP + WMS, sincronizados por ETL noturno (hoje via n8n) |
| Usuários | Equipe de PCP/produção (operação diária), gestão (dashboard executivo) |

## 3. Problema que o sistema resolve

Antes do sistema, a decisão de produção era manual, baseada em planilhas e na percepção da equipe. Consequências documentadas:

- Rupturas frequentes de itens classe A (alta rotação);
- Capital parado em itens de baixíssimo giro (~40% dos produtos com estoque excessivo em auditorias);
- Nenhum tratamento sistemático de sazonalidade (produção de dezembro planejada com média anual);
- Produtos "mortos" mantidos em linha por falta de análise de ciclo de vida.

## 4. Proposta de valor / objetivos de negócio

| Objetivo | Métrica de sucesso (declarada no projeto atual) |
|---|---|
| Reduzir tempo de análise manual | -80% |
| Eliminar rupturas de itens A/B | -95% de faltas |
| Otimizar capital de giro | -30% de estoque excessivo |
| Automatizar o processamento | 100% (pipeline diário sem intervenção) |
| ROI estimado | ~R$ 650 mil/ano |

## 5. Personas

### 5.1 Analista de PCP (usuário principal)
- Abre o sistema todo dia entre 5h e 6h, após o processamento noturno.
- Consome: central de alertas (o que produzir hoje), tabela de estoque com filtros, detalhes de produto.
- Ações: marcar produtos para produção, gerar solicitação de produção com quantidade sugerida, exportar listas.

### 5.2 Gestor de Produção / Diretoria
- Consome: dashboard executivo (cobertura média, distribuição ABC, alertas críticos, metas de estoque físico por classe).
- Ações: acompanhar tendências (gráfico de estoque 30 dias por classe), validar sugestões de fora de linha.

### 5.3 Usuário de consultas ad-hoc
- Usa o **Chat IA** para perguntas em linguagem natural ("quais os 10 produtos mais críticos da classe A?", "como foram as vendas da semana?").

## 6. Escopo funcional (módulos)

| # | Módulo | Status no sistema atual | Levar para o novo? |
|---|---|---|---|
| 1 | Pipeline diário de processamento (ETL + cálculo) | ✅ Produção | **Sim — núcleo do sistema** |
| 2 | Classificação ABC+F+D+N | ✅ Produção (v3.0) | **Sim** |
| 3 | Parâmetros de estoque por produto | ✅ Produção | **Sim** |
| 4 | Sazonalidade dinâmica | ✅ Produção | **Sim** |
| 5 | Alertas de produção | ✅ Produção | **Sim** |
| 6 | Análise fora de linha (ciclo de vida) | ✅ Produção (aplicação manual) | **Sim**, com workflow de aprovação |
| 7 | Dashboard executivo | ✅ Produção | **Sim** |
| 8 | Gestão de estoque (tabela + filtros + export) | ✅ Produção | **Sim** |
| 9 | Página de detalhes do produto + insights ML | ✅ Produção (parte simulada) | **Sim**, completando as ações reais |
| 10 | Chat IA com function calling | ✅ Produção | **Sim** |
| 11 | Análise OpenAI por produto | ✅ Produção | **Sim** (opcional na fase 1) |
| 12 | Analytics avançado | 🟡 Placeholder | Roadmap |
| 13 | Configurações (thresholds, usuários) | 🟡 Placeholder | **Sim — promover a requisito** (hoje as regras são hardcoded) |
| 14 | Integração com agendamento de produção (schema `public`) | 🟡 Não integrado | Roadmap |

## 7. Visão de arquitetura (conceitual, agnóstica de stack)

```
┌─────────────┐   ETL noturno    ┌──────────────────────┐
│  ERP / WMS  │ ───────────────► │  Banco de dados      │
└─────────────┘  (vendas dia +   │  - vendas_dia        │
                  snapshot       │  - estoque_snapshot  │
                  estoque)       └─────────┬────────────┘
                                           │ disparo pós-carga
                                           ▼
                              ┌─────────────────────────────┐
                              │  MOTOR PCP (job diário)     │
                              │  1. Sazonalidade (mensal)   │
                              │  2. Classificação ABCFDN    │
                              │  3. Parâmetros de estoque   │
                              │  4. Alertas de produção     │
                              │  5. Análise fora de linha   │
                              └─────────┬───────────────────┘
                                        │ tabelas derivadas + views
                                        ▼
                  ┌────────────────────────────────────────┐
                  │  API / camada de leitura agregada      │
                  └───────┬──────────────────┬─────────────┘
                          ▼                  ▼
                  ┌──────────────┐   ┌───────────────────┐
                  │  Frontend    │   │  Chat IA (LLM com │
                  │  (SPA)       │   │  function calling)│
                  └──────────────┘   └───────────────────┘
```

Princípios para o novo projeto (ver documento 08 para o racional):

1. **Motor de cálculo único e versionado** — uma só implementação de cada regra (hoje há regras duplicadas entre SQL, edge functions e frontend, com valores divergentes).
2. **Frontend burro em regra de negócio** — consome valores pré-calculados; não recalcula status/metas localmente.
3. **Pipeline idempotente** — reprocessar uma data substitui os resultados daquela data sem efeitos colaterais.
4. **Histórico com retenção definida** — o sistema atual acumulou 33,7 milhões de linhas em uma tabela de análise por falta de expurgo.
5. **Segurança por padrão** — RLS/autorização desde o primeiro dia (hoje as tabelas estão expostas com a chave anônima).
