# 06 — IA e Insights

> Três capacidades de inteligência existem no sistema atual: (1) Chat IA conversacional, (2) análise pontual de produto via OpenAI, (3) insights estatísticos/ML calculados no frontend. Este documento especifica o comportamento esperado de cada uma no novo sistema.

## 1. Chat IA conversacional

### 1.1 Conceito
Assistente de PCP que responde perguntas em linguagem natural **consultando dados reais** via *function calling* — nunca inventa números. Fluxo **iterativo**: uma consulta por vez, com sugestões de próximos passos.

### 1.2 Arquitetura atual (referência)
- Edge function (`chat-ia-pcp`) com OpenAI `gpt-4-0125-preview`, `temperature 0.3`, `max_tokens 1000`.
- Histórico enviado: últimas **3 mensagens** (controle de custo de tokens).
- Duas chamadas por turno quando há function call: 1ª decide a ferramenta → executa consulta → 2ª gera a resposta com os dados.
- Acesso a dados via API REST com schema dedicado e chave de serviço (somente leitura).

> No novo projeto o modelo/provider é livre; o **contrato funcional** abaixo é o requisito.

### 1.3 Ferramentas expostas à IA (contrato)

| Ferramenta | Parâmetros | Retorno (limites) |
|---|---|---|
| `consultar_resumo_vendas` | `periodo`: hoje/semana/mes/ano | agregado + top 50 |
| `consultar_resumo_estoque` | — | agregado + top 50 |
| `consultar_alertas` | `limite` (máx 10) | alertas ordenados por cobertura asc |
| `consultar_produto_especifico` | `codigo` | snapshot (10) + vendas recentes (15) |
| `consultar_top_produtos` | `tipo`: vendas/estoque, `limite` (máx 10) | ranking |
| `buscar_variacoes_produto` | `codigoBase` | até 20 variações |

Extensões desejáveis (já prototipadas no legado): `consultar_abc_detalhado`, `consultar_cobertura_produtos`, `consultar_analise_fora_linha`, `executar_analise_sql` (**somente SELECT**, máx 100 linhas, com bloqueio de DDL/DML).

### 1.4 Conhecimento de domínio no system prompt
O prompt deve ensinar à IA as regras canônicas do doc 02 (status, metas ABC 45/30/15/10/5/20, limiares críticos 15/10/5, fórmulas de cobertura e reposição, vocabulário "Produzir"), para que as respostas usem a mesma linguagem do sistema.

### 1.5 Persistência e UX
- Conversas e mensagens persistidas por usuário (com RLS), incluindo: ferramenta chamada, argumentos, dados usados, tokens, tempo de resposta.
- Título automático da conversa (primeira mensagem) e tags automáticas por assunto.
- Favoritos, busca por texto/tag/data, exclusão.
- Resposta sempre acompanhada de **2–3 sugestões de próximo passo** clicáveis.
- Perguntas rápidas pré-definidas na tela inicial do chat.
- Transparência: indicar qual consulta foi executada para gerar a resposta.

### 1.6 Limites e segurança
- IA **não escreve** em nenhuma tabela.
- Limites de linhas por consulta (50–100) e truncamento de payload para controlar tokens.
- Timeout por turno; retry com backoff; mensagem de erro amigável.

---

## 2. Análise OpenAI por produto ("Solicitação de Produção Inteligente")

### 2.1 Conceito
No detalhe do produto, o usuário pode pedir uma análise crítica gerada por LLM que valida/ajusta a recomendação calculada localmente.

### 2.2 Contrato de entrada
```json
{
  "product":  { ...dados completos do produto (estoque, cobertura, classe, parâmetros)... },
  "insights": { ...alertas e recomendações do motor local (doc 02 §7)... }
}
```

### 2.3 Contrato de saída
```json
{
  "recommendedQuantity": 1234,
  "priority": "alta | média | baixa",
  "reasoning": ["mínimo 4 justificativas baseadas nos dados"],
  "confidence": 0.85,
  "riskFactors": ["..."],
  "opportunities": ["..."],
  "detailedAnalysis": "texto 200+ palavras",
  "alternativeStrategies": [
    { "strategy": "...", "quantity": 0, "reasoning": "..." }  // exatamente 3
  ]
}
```

### 2.4 Regras do prompt (negócio)
- Metas de cobertura por classe: **A 45 / B 30 / C 15 / D 10 / F 5 / N 20 dias**.
- Contexto: ~90% produção própria; lead time 7–15 dias; foco em "produzir".
- Considerar sazonalidade (fator do mês) e qualidade do histórico (CV, outliers, dias com venda).
- Parâmetros do legado: `gpt-4`, `temperature 0.3`, `max_tokens 2000`, resposta JSON pura.
- **Fallback obrigatório:** se a chamada falhar, exibir a análise local (motor doc 02 §7) sem quebrar o fluxo.

---

## 3. Insights estatísticos por produto (motor local)

> No legado roda no frontend (`mlAlgorithms.ts`); no novo sistema **deve ir para o backend** (consistência e testabilidade), mantendo os mesmos algoritmos.

### 3.1 Algoritmos utilizados
| Técnica | Uso |
|---|---|
| Regressão linear simples | tendência de vendas (slope + correlação) |
| Médias móveis (7d) | baseline de demanda |
| Suavização exponencial | série suavizada |
| Decomposição sazonal (dia da semana) | padrão semanal + força sazonal |
| Remoção de outliers IQR (1.5×) | limpeza da série |
| Correlação de Pearson | confiança da tendência |
| MAPE/RMSE | qualidade da previsão |

### 3.2 Previsão de demanda (7 dias)
```
predicted(i) = trend(i) × (0.4 × |correlação|)
             + último_suavizado × fator_sazonal_dia × (0.3 × força_sazonal)
             + baseline_MA7 × 0.3

confiança = MIN((|correlação| + confiança_sazonal) / 2, 0.95)
```

### 3.3 Alertas inteligentes gerados

| Alerta | Condição | Severidade |
|---|---|---|
| Ruptura crítica | cobertura < 7 dias | crítico |
| Meta ABC não atingida | déficit > 30% da meta (crítico se > 70%) | atenção/crítico |
| Ruptura do disponível | previsão 7d > qtd_disponível | atenção |
| Demanda em alta/queda | variação prevista > |20%| | informativo |
| Pico sazonal próximo | força sazonal > 0.5 e dias com previsão > 130% da média | informativo |
| Excesso de estoque | cobertura > 3 meses + tendência de queda | atenção |
| Qualidade de dados | < 50% dos dias com venda no ano | informativo |

Categorias: `ruptura · excesso · demanda · sazonalidade · padrão`. Severidades: `crítico · atenção · informativo · positivo`.

### 3.4 Recomendações
Regra completa no doc 02 §7.3 (escalonamento por criticidade sobre a meta ABC).

### 3.5 O que NÃO levar do legado
- Correlações entre produtos **mockadas** (aleatórias) — implementar de verdade ou cortar.
- Processamento no navegador — mover para o backend, com cache por produto/dia.

---

## 4. Resumo de requisitos de IA para o novo sistema

1. Chat com function calling sobre dados reais, persistência por usuário e fluxo iterativo (§1).
2. Análise de produto via LLM com contrato JSON fixo e fallback local (§2).
3. Motor de insights estatísticos no backend com os algoritmos de §3, alimentando a página de produto e os alertas inteligentes.
4. Todas as chamadas LLM com: registro de tokens/custo, timeout, retry, e chave em secret manager.
5. Modelos/providers configuráveis (não hardcoded como no legado).
