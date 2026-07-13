# 09 — Roadmap de Implementação Sugerido

> Sequência proposta para a reconstrução, ordenada por dependência e valor. Cada fase tem critérios de aceite objetivos. Estimativas de esforço ficam a cargo do novo time (stack ainda não definida).

## Fase 0 — Fundações
**Objetivo:** ambiente e contratos prontos.

- Projeto/banco dedicado com ambientes dev/staging/prod.
- Esquema de migrations versionado; CI com testes.
- Modelo de dados de entrada (`vendas_dia`, `estoque_snapshot`) + autenticação/papéis + RLS deny-by-default.
- **Migração dos dados históricos**: `vendas_dia` completa (jan/2024→) e snapshots de estoque do legado.
- Configuração central de constantes de negócio (doc 02 §11) lida pelo motor.

✅ **Aceite:** dados históricos migrados e batendo contagens com o legado; usuário autenticado consegue ler, anônimo não consegue nada.

## Fase 1 — Pipeline + Motor PCP (núcleo)
**Objetivo:** o cálculo diário funcionando com paridade ao legado.

- ETL diário (ERP → vendas/snapshot) com validações bloqueantes e notificação de falha (doc 05).
- Motor com os 4 módulos: classificação ABCFDN → parâmetros → alertas → fora de linha (doc 02).
- Sazonalidade dinâmica com atualização mensal automática.
- Tabela de execuções + painel de status do pipeline.
- Testes de paridade (doc 08 §3–4) contra o sistema atual rodando em paralelo.

✅ **Aceite:** 7 dias consecutivos de execução em staging com paridade de classificação/alertas vs legado (tolerância de arredondamento); reprocesso de data idempotente.

## Fase 2 — Telas operacionais
**Objetivo:** analista de PCP trabalha 100% no sistema novo.

- Central de Alertas (fila de produção do dia) — doc 03 §5.
- Gestão de Estoque (tabela + filtros avançados + filtros salvos + export completo) — doc 03 §3.
- Detalhe do Produto (métricas, gráficos 90d, regras da classe) — doc 03 §4.
- **Solicitação de produção real** (persistida, com status) — substitui a simulação do legado.

✅ **Aceite:** analista executa a rotina diária inteira (triagem de alertas → análise → solicitação) sem recorrer ao sistema antigo.

## Fase 3 — Dashboard executivo + ABC
- Dashboard com carregamento progressivo (doc 03 §2) e metas de estoque físico por classe.
- Página de Classificação ABC com Pareto e exportação.
- Workflow de fora de linha com aprovação (gestor) e trilha de auditoria.

✅ **Aceite:** gestor acompanha a operação sem planilhas; sugestões de fora de linha aplicadas com auditoria.

## Fase 4 — IA
- Motor de insights estatísticos no backend (previsão 7/30d, alertas inteligentes) — doc 06 §3.
- Chat IA com function calling e histórico persistido — doc 06 §1.
- Análise OpenAI por produto com fallback local — doc 06 §2.

✅ **Aceite:** chat responde as 6 perguntas rápidas com dados reais; análise de produto retorna o contrato JSON completo.

## Fase 5 — Cut-over e desligamento do legado
- Período de operação paralela (2–4 semanas) com comparação diária automática dos resultados.
- Treinamento da equipe; migração dos filtros salvos/preferências.
- Redirecionamento do ETL definitivo; congelamento do sistema antigo (somente leitura) e posterior desligamento.

✅ **Aceite:** 100% da operação no novo sistema por 2 semanas sem incidente bloqueante.

## Pós-MVP (backlog priorizado — doc 08 §5)
1. Dimensão financeira (custo/preço → capital parado, ROI).
2. Alertas preditivos 7/15/30 dias + notificações proativas (WhatsApp/e-mail) para classe A.
3. Índice de saúde do estoque (0–100).
4. Integração com agendamento de produção (injetoras/ordens — hoje em schema separado não integrado).
5. Analytics avançado (forecast 30/60/90, comparativos YoY).
6. App/PWA mobile para chão de fábrica.

## Riscos principais

| Risco | Mitigação |
|---|---|
| Divergência de regra na reimplementação | Doc 02 como contrato + testes de paridade da Fase 1 |
| Qualidade do ETL (contrato com ERP não documentado formalmente) | Validar contrato do doc 05 §2 com amostras reais antes da Fase 1 |
| Dependência do n8n/ERP | Health checks + alertas + plano de reprocesso |
| Adoção da equipe | Operação paralela na Fase 5 + treinamento |
| Custo de IA | Telemetria de tokens desde a Fase 4; limites por usuário |
