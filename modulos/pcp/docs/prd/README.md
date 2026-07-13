# PRD — SuperCopo PCP 2.0

> **Product Requirements Document** para reconstrução do sistema de Planejamento e Controle de Produção (PCP) da SuperCopo, do zero, preservando as regras de negócio validadas no sistema atual (SuperCopo PCP AI) e corrigindo os débitos arquiteturais conhecidos.

**Data de elaboração:** Junho/2026
**Fontes:** código-fonte do projeto atual, banco de dados Supabase em produção (`hrggrfatvxzvahbiityd`, schema `supercopo_pcp`), ~64 documentos técnicos da pasta `documentacao/`.

---

## Como usar esta documentação

Esta pasta é **autossuficiente**: a equipe de desenvolvimento do novo projeto não precisa ler o código legado para implementar o sistema. As regras de negócio aqui descritas foram extraídas **diretamente do banco de produção** (funções SQL, views e dados reais), que é a fonte canônica quando a documentação histórica diverge.

| Documento | Conteúdo | Público |
|---|---|---|
| [01 — Visão Geral do Produto](./01-visao-geral-produto.md) | O que é o sistema, objetivo de negócio, contexto operacional, métricas de sucesso | Todos |
| [02 — Regras de Negócio](./02-regras-de-negocio.md) | **Documento central.** Todas as fórmulas e regras vigentes com valores exatos: classificação ABC+F+D+N, parâmetros de estoque, status, cobertura, alertas, fora de linha, sazonalidade, reposição | Dev backend + Produto |
| [03 — Páginas e Funcionalidades](./03-paginas-e-funcionalidades.md) | Especificação funcional de cada página: dashboard, estoque, detalhes do produto, alertas, ABC, chat IA | Dev frontend + Produto + UX |
| [04 — Modelo de Dados](./04-modelo-de-dados.md) | Tabelas, views, funções do banco atual; dados populados; o que reaproveitar e o que reconstruir | Dev backend / DBA |
| [05 — Pipeline de Dados (ETL)](./05-pipeline-dados-etl.md) | Integração ERP/WMS → n8n → banco; processamento diário; contratos de dados | Dev backend / DevOps |
| [06 — IA e Insights](./06-ia-e-insights.md) | Chat IA conversacional, análise OpenAI por produto, algoritmos de ML do frontend | Dev fullstack + Produto |
| [07 — Requisitos Não Funcionais](./07-requisitos-nao-funcionais.md) | Segurança (RLS!), performance, autenticação, observabilidade, disponibilidade | Dev + DevOps |
| [08 — Inconsistências do Legado e Decisões para o Novo Projeto](./08-inconsistencias-e-melhorias.md) | Divergências encontradas no sistema atual e a regra canônica adotada; melhorias obrigatórias | Todos |
| [09 — Roadmap de Implementação](./09-roadmap-implementacao.md) | Fases sugeridas, priorização, critérios de aceite por fase | Gestão + Dev |

---

## Resumo executivo

O **SuperCopo PCP** automatiza decisões de **produção e reposição de estoque** de uma fábrica com ~90% de produção própria (copos e produtos plásticos personalizáveis), com base em:

1. **Histórico de vendas diárias** (desde jan/2024) sincronizado do ERP via ETL noturno;
2. **Snapshot diário de estoque** (disponível, reserva, total) do WMS/ERP;
3. **Classificação automática de 6 níveis** (A/B/C/D/F/N) por curva de Pareto sobre 18 meses de vendas;
4. **Parâmetros estatísticos por produto** (média diária sem outliers, desvio, estoque de segurança, estoque recomendado) recalculados diariamente;
5. **Sazonalidade dinâmica** (fatores mensais recalculados automaticamente a partir das vendas do ano anterior);
6. **Alertas de produção priorizados** (CRÍTICO/ALTO/MÉDIO) gerados diariamente;
7. **Gestão de ciclo de vida** (sugestões automáticas de tirar/voltar produtos de linha);
8. **IA conversacional** (chat com function calling sobre os dados) e análise pontual de produto via OpenAI.

### Números do sistema em produção (jun/2026)

| Métrica | Valor |
|---|---|
| Produtos monitorados (snapshot atual) | ~2.838 códigos de estoque |
| Histórico de vendas | jan/2024 → hoje (~106 mil registros diários consolidados) |
| Snapshots de estoque | diários desde jun/2025 (~871 mil registros) |
| Distribuição de classes (atual) | A=165 · B=346 · C=671 · D=1.012 · F=177 · N=9 |
| Volume 18m classe A | ~5,16 milhões de unidades (≈80% do volume total) |
| Produtos com parâmetros calculados | 2.346 |
| Alertas ativos gerados por dia | ~150–400 |

### Premissas da reconstrução (definidas pelo negócio)

- **Regras de negócio idênticas** às vigentes (documento 02 é contrato).
- **Stack livre**: o novo time escolhe linguagens/frameworks; este PRD é agnóstico de tecnologia, descrevendo o quê, não o como — exceto onde a arquitetura atual é um requisito (ex.: processamento diário em lote).
- **Banco de dados refeito do zero**, usando o modelo do documento 04 como referência do domínio (entidades e dados), não como DDL a copiar.
- **Reaproveitar o máximo possível** de conceitos, telas e regras já validados; descartar os débitos listados no documento 08.
