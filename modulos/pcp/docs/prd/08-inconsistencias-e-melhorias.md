# 08 — Inconsistências do Legado e Decisões para o Novo Projeto

> Catálogo das divergências encontradas entre código SQL, edge functions, frontend e documentação do sistema atual — com a **decisão canônica** adotada neste PRD para cada uma. Este documento evita que o novo time reimplemente ambiguidades.

## 1. Divergências de regra de negócio (e a regra que vale)

| # | Tema | Versões encontradas no legado | ✅ Decisão canônica (PRD) |
|---|---|---|---|
| 1 | **Fatores de estoque ABC** | a) 1.3/1.1/0.9 (função antiga `calcular_abc`); b) 1.2/1.0/0.8 + D 0.3, F 0.1, N 0.8 (v3.0 em produção) | **b)** — doc 02 §2.5 |
| 2 | **Janela da curva ABC** | 12 meses (docs antigas) vs 18 meses/540 dias (v3.0) | **18 meses** |
| 3 | **Fórmula do estoque recomendado** | a) base 15 dias + segurança z=1.96 (v2.4); b) base 15 dias × sazonal × fator ABC + segurança z=1.28, teto 60d (módulo vigente); c) meta ABC 45/30/15 × sazonal + segurança (recalibração) | **Unificar em c)** com teto de 60 dias e segurança estatística de b) — doc 02 §3.6 |
| 4 | **Z-score do estoque de segurança** | 1.96 (docs/v2.4) vs 1.28 (módulo em produção) | **1.28** (90% nível de serviço) — configurável |
| 5 | **Status de estoque** | a) faixas fixas 10/15/25/30 dias (`v_cobertura_v2`, prompt do chat, labels do dashboard); b) hierarquia com criticidade por classe A≤15/B≤10/C≤5 (`v_produtos_ativos`); c) 7/15/45/90 (hook `useEstoque`); d) 5/15/45/90 (hook `useDashboard`) | **b)** — doc 02 §5.2. Faixas fixas somente como visualização da distribuição agregada |
| 6 | **Metas de cobertura por classe** | a) A=45/B=30/C=15/D=10/F=5/N=20 (vigente); b) A=30/B=45/C=60 (versão intermediária com erro conceitual — meta invertida) | **a)** |
| 7 | **Alertas: limiares** | a) % do recomendado 20/50/80 + elevação classe A (módulo vigente); b) cobertura <7/<10/<15 dias (docs antigas/prompt do chat) | **a)** para geração; cobertura exibida como dado complementar |
| 8 | **Reposição: estoque base** | `qtd_disponivel` (alertas) vs `qtd_estoque` (UI "Reposição") | **`qtd_disponivel`** (o reservado já tem destino). Exibir ambos na UI |
| 9 | **Pipeline** | monolítico `processar-dados-pcp` (v2.5) vs modular `processar-pcp-modular` (vigente, chamado pelo n8n) | **modular** — doc 05 |
| 10 | **Metas Pareto** | A≤80%, B≤95% consistente em todas as versões | manter |

## 2. Defeitos estruturais a não repetir

| # | Defeito | Correção no novo projeto |
|---|---|---|
| 1 | Prioridade do alerta gravada no campo `configuracao` | Coluna própria `prioridade` |
| 2 | `analise_fora_linha` com **33,7 milhões de linhas** (regravação diária sem expurgo) | Máquina de estados + retenção 90 dias (doc 04 §3.4) |
| 3 | Coluna `volume_12m` contém volume de 18 meses | Nome honesto (`volume_janela`) + metadado da janela |
| 4 | Defaults grosseiros para produto sem histórico (média=50, min=750, seg=250) | Status `SEM_HISTORICO_CONFIAVEL` + parâmetros configuráveis + tratamento de classe N |
| 5 | Regras duplicadas em 3 camadas (SQL, edge function, frontend) com valores divergentes | **Motor único no backend**; frontend não recalcula nada |
| 6 | Constantes hardcoded espalhadas | Configuração central editável com auditoria (doc 02 §11) |
| 7 | RLS desabilitado em todas as tabelas de negócio | Deny-by-default (doc 07 §1) |
| 8 | 20+ migrations existem só no banco remoto | Todas as migrations versionadas no repositório |
| 9 | Card do dashboard com números hardcoded no JSX | Tudo vem da API |
| 10 | Ações de produção simuladas (setTimeout) e fila em localStorage | Solicitações persistidas com workflow real (doc 03 §4.3) |
| 11 | Banco compartilhado com ~10 sistemas | Projeto/banco dedicado (doc 07 §4) |
| 12 | Sem dimensão financeira (`valor_estoque` = 0 sempre) | Cadastro de custo/preço por produto; priorização também por valor |
| 13 | Versões duplicadas de funções (v1/v2, duas `aplicar_sugestoes_fora_linha`) | Uma versão de cada conceito; remoção da anterior na mesma migration |
| 14 | Documentação volumosa porém divergente do código | Documentação viva: doc 02 §11 como config executável + testes que validam as constantes |

## 3. Dados de referência para validação (paridade com o legado)

Distribuição esperada com os dados atuais (jun/2026) ao implementar a classificação — usar como teste de aceitação aproximado:

| Classe | Produtos | Volume (janela 18m) |
|---|---|---|
| A | 165 | ~5.165.109 (≈80%) |
| B | 346 | ~970.436 (≈15%) |
| C | 671 | ~322.719 (≈5%) |
| D | 1.012 | 0 |
| F | 177 | 0 |
| N | 9 | ~1.102 |

Fatores sazonais calculados sobre 2025: Jan 1.25 · Fev 0.99 · Mar 1.00 · Abr 0.87 · Mai 0.81 · Jun 0.62 · Jul 0.64 · Ago 0.63 · Set 0.71 · Out 0.90 · Nov 0.67 · Dez 2.00.

## 4. Testes de regressão de regra (obrigatórios)

1. **Produtos de referência:** a documentação do legado usa os códigos `6797`, `10001` e `10473` como casos validados manualmente pelo negócio — reproduzir os cálculos deles no novo motor e comparar com o sistema atual antes do cut-over.
2. **Paridade de pipeline:** rodar o novo motor sobre um dump de `vendas_dia` + `estoque_snapshot` e comparar classificação, parâmetros e alertas com a saída do sistema atual na mesma data (tolerância: arredondamentos).
3. **Propriedades invariantes:** soma dos percentuais de Pareto = 100; nenhum produto com duas classes no mesmo dia; `qtd_sugerida ≥ 0`; produto fora de linha nunca gera alerta; cobertura sentinela 999 nunca entra em médias.

## 5. Melhorias desejadas (não regressões — backlog)

- Dimensão financeira (capital parado, ROI por decisão).
- Workflow completo de solicitação de produção (status, aprovação, integração com agenda de produção).
- Alertas preditivos (ruptura prevista em 7/15/30 dias usando o motor de previsão do doc 06 §3).
- Índice de saúde do estoque (0–100) por classe e geral.
- Integração da aplicação de "fora de linha" com o ERP (hoje é só registro local).
- Notificações proativas (WhatsApp/e-mail) para alertas críticos de classe A.
