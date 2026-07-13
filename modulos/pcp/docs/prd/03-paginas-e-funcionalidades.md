# 03 — Páginas e Funcionalidades (Especificação Funcional)

> Especificação das telas do sistema, baseada no produto em produção. O novo projeto deve entregar **a mesma capacidade funcional**; layout e componentes podem ser redesenhados. Onde o legado tem funcionalidade simulada/incompleta, está marcado como **[COMPLETAR]** — vira requisito real no novo sistema.

## Mapa de navegação

```
Login (gate de autenticação)
└── Aplicação autenticada (layout: sidebar + header + conteúdo)
    ├── /dashboard          Dashboard executivo (página inicial)
    ├── /estoque            Gestão de estoque (tabela paginada + filtros)
    │   └── /estoque/:codigo  Detalhes do produto
    ├── /alertas            Central de alertas de produção
    ├── /abc                Classificação ABC
    ├── /ai-chat            Chat IA
    ├── /analytics          [ROADMAP] Analytics avançado
    └── /configuracoes      [PROMOVER A REQUISITO] Configurações
```

---

## 1. Login

- Autenticação por e-mail/senha.
- Sessão persistente com refresh automático.
- **[NOVO]** Gestão de usuários e papéis (analista, gestor, admin) — o legado não tem papéis.
- **[REMOVER]** Login demo hardcoded (`demo@supercopo.com`) — não levar para produção nova.

---

## 2. Dashboard (`/dashboard`)

**Objetivo:** visão executiva em até 5 segundos: saúde do estoque, criticidade e tendência.

### 2.1 Seções e dados

| Seção | Conteúdo | Origem dos dados |
|---|---|---|
| Gráfico de estoque 30 dias | Série diária de estoque total por classe (A/B/C/D/F/N) — área/linhas | Agregação do histórico de snapshots |
| Painel de metas ABC | Participação atual vs meta do estoque físico por classe (A 50% / B 30% / C 20% / D 0%) com indicador de meta atingida (±3 p.p.) | Distribuição física por classe |
| Card Produtos | Total, ativos (%), sem estoque, fora de linha, "em linha sem estoque" | Resumo de produtos |
| Card Alertas Críticos | Total de críticos, % do catálogo, breakdown por classe (com limiares A≤15d, B≤10d, C≤5d) | Alertas detalhados |
| Card Cobertura | Cobertura média (dias + status) e distribuição por status (crítico/baixo/adequado/alto/excessivo/sem histórico/sem estoque) | Distribuição de cobertura |
| Card Cobertura por classe | Cobertura média de A, B e C separadas, com status | Médias por classe |
| Top produtos ABC | Maiores e menores estoques por classe (lazy load ao entrar na viewport) | Top N otimizado |
| Alertas recentes | Lista dos 5 alertas mais urgentes com link para o produto | Alertas completos |
| Distribuição ABC | Quantidade de produtos e % por classe | Classificação atual |

### 2.2 Comportamento
- **Carregamento progressivo por seção** (cada bloco busca seus dados de forma independente, com skeleton) — requisito de UX validado; o dashboard inteiro não pode travar em uma query lenta.
- Cache de leitura ~5 min; sem refetch agressivo ao focar a janela.
- Cores de criticidade nos cards conforme thresholds do doc 02 §9.2.

### 2.3 Defeitos do legado a corrigir
- Card "Fora de Linha ABC" exibia **valores hardcoded** — tudo deve vir do backend.
- Três conjuntos de faixas de cobertura coexistem em tooltips/labels — usar somente a regra canônica (doc 02 §5).

---

## 3. Gestão de Estoque (`/estoque`)

**Objetivo:** ferramenta de trabalho diária do analista — encontrar produtos, avaliar situação e agir.

### 3.1 Cards de resumo (clicáveis = aplicam filtro)

Total · Críticos · Baixos · Adequados · Altos · Excessivos · Sem estoque · Fora de linha · Com outliers · **Total a produzir** (soma das sugestões) · **Cobertura média** · Distribuição ABC (A/B/C/D/F/N).

### 3.2 Filtros avançados

| Tipo | Filtros |
|---|---|
| Busca textual | nome do produto, código, SKU (busca parcial, case-insensitive) |
| Seleção | classe (A/B/C/D/F/N), status de estoque, cor/configuração |
| Faixas | cobertura (dias min/max), quantidade em estoque (min/max), volume 12m (min/max) |
| Switches | apenas sem estoque · apenas críticos · apenas com sugestão de produção · apenas com outliers · apenas fora de linha · apenas sem classificação |
| Ordenação | por qualquer coluna, asc/desc |
| Persistência | **filtros salvos nomeados** por usuário (no legado: localStorage; no novo: persistir no backend por usuário) |

### 3.3 Tabela (colunas)

Código · Produto (+cor extraída da configuração) · Estoque · Disponível · Reserva · Cobertura (dias) · Status (semáforo) · Classe · Volume 12m · Demanda diária · Mínimo · **Recomendada** · **Sugestão de produção** · mini-sparkline de tendência · menu de ações.

### 3.4 Paginação
50 / 100 / 500 / 1000 itens por página, com contagem total. A consulta deve ser paginada no servidor (o catálogo tem ~2.800 itens e cresce).

### 3.5 Ações
- **Ver detalhes** → `/estoque/:codigo`.
- **Exportar** CSV/JSON (UTF-8 com BOM para Excel BR). **[COMPLETAR]** exportar o resultado completo do filtro, não só a página atual.
- **Marcar para produção** (seleção múltipla → fila de produção). **[COMPLETAR]** no legado grava em localStorage; no novo deve criar registro de solicitação no backend.
- Atualizar (refetch manual).

---

## 4. Detalhes do Produto (`/estoque/:codigo`)

**Objetivo:** análise profunda de um produto + geração de solicitação de produção.

### 4.1 Seções

| Seção | Conteúdo |
|---|---|
| Cabeçalho | Nome, SKU, código, badge da classe, badge do status, regra da classe aplicada (meta de dias, faixa adequada, justificativa) e botão voltar |
| Métricas | Estoque (total/disponível/reserva), cobertura em dias, demanda média diária, estoque de segurança, mínimo, recomendado, sugestão de produção, qualidade dos dados (dias com venda / outliers / CV) |
| Gráficos | Vendas diárias (90 dias) e evolução do estoque (90 dias) |
| Insights inteligentes | Alertas inteligentes, previsão de demanda 7/30 dias, padrão sazonal, tendência, recomendações com quantidade/timing/prioridade (ver doc 06) |
| Centro de comando (ações) | Gerar **Solicitação de Produção Inteligente** (regra doc 02 §7.2) com edição de quantidade, prioridade e justificativas; exportar relatório do produto; ajustar parâmetros **[COMPLETAR]**; sincronizar com ERP **[COMPLETAR]** |

### 4.2 Estados obrigatórios
Skeleton de carregamento · erro com retry · produto não encontrado.

### 4.3 Requisito de integração **[COMPLETAR]**
No legado, a solicitação de produção é **simulada** (setTimeout). No novo sistema ela deve:
1. Persistir a solicitação (produto, quantidade, prioridade, prazo, solicitante, justificativas);
2. Alimentar a fila/agenda de produção;
3. Permitir acompanhamento de status (pendente → aprovada → em produção → concluída).

---

## 5. Central de Alertas (`/alertas`)

**Objetivo:** fila de trabalho diária — o que produzir hoje, em ordem de urgência.

### 5.1 Conteúdo
- Cards de resumo: Total, Críticos, Altos, Médios.
- Lista/tabela de alertas ordenada por urgência (prioridade → classe → quantidade sugerida; doc 02 §6.5).
- Cada alerta exibe: produto (+cor), código, prioridade, estoque disponível/reserva, cobertura em dias, classe, demanda média, estoque recomendado, **quantidade sugerida**, flag de **ruptura iminente** (cobertura ≤ 3 dias ou sem estoque), link para detalhes.
- Filtros: prioridade, busca textual, cobertura máxima.

### 5.2 Tempo real
Atualização automática quando novos alertas são gravados (no legado: subscription realtime na tabela de alertas → invalida cache). Requisito: a tela refletir o processamento diário sem reload manual.

---

## 6. Classificação ABC (`/abc`)

**Objetivo:** entendimento da curva de Pareto e consulta da classificação.

- Card com total de produtos classificados e distribuição por classe.
- **Gráfico de Pareto**: top 20 produtos por volume (barras) + % acumulado (linha).
- Tabela completa: código, produto, classe, volume (janela ABC), % acumulado, fator de estoque, estoque atual, status.
- Busca local por código/nome.
- **[COMPLETAR]** Exportação (botão existia sem implementação).
- **[CORRIGIR]** Garantir 1 linha por produto usando apenas a classificação mais recente (legado tinha duplicatas históricas).

---

## 7. Chat IA (`/ai-chat`)

**Objetivo:** consultas em linguagem natural sobre os dados de PCP. Especificação completa no doc 06.

### 7.1 UI
- Conversa com histórico persistido (conversas, mensagens, tags automáticas, favoritos, busca).
- 6+ perguntas rápidas pré-definidas (resumo de vendas, estoque, alertas críticos, top produtos...).
- Sugestões de próximos passos (2–3) após cada resposta (fluxo iterativo).
- Retry em falha; indicador de digitação; exibição de qual consulta foi executada (transparência).

---

## 8. Configurações (`/configuracoes`) — **[PROMOVER A REQUISITO]**

Placeholder no legado. No novo sistema, é a tela que elimina o hardcode de regras:

1. **Parâmetros de negócio editáveis** (todas as constantes do doc 02 §11) com trilha de auditoria (quem mudou, quando, valor anterior).
2. **Fatores sazonais**: visualizar fatores vigentes, histórico, desvio real × previsto; permitir override manual com justificativa.
3. **Gestão de usuários e papéis**.
4. **Preferências de exibição** (colunas visíveis, página inicial, tamanho de página padrão).

---

## 9. Analytics (`/analytics`) — **[ROADMAP]**

Placeholder no legado. Backlog priorizado no doc 09 (métricas financeiras, forecast 30/60/90 dias, índice de saúde do estoque 0–100, comparativos YoY).

---

## 10. Requisitos transversais de UX

| Requisito | Detalhe |
|---|---|
| Responsivo | Desktop-first, funcional em tablet/mobile |
| Tema | Claro/escuro |
| Performance percebida | Skeletons por seção; nenhuma tela "toda branca" esperando uma única query |
| Acessibilidade de dados | Tooltips explicando cada métrica e a regra de cálculo (a equipe valorizou muito isso no legado) |
| Cores semânticas | Status de estoque com semáforo consistente em todas as telas; cores fixas por classe ABC |
| Nomenclatura | Doc 02 §10 (Produzir, Recomendada, Sem histórico, etc.) |
| Exportações | CSV UTF-8 BOM (Excel BR), JSON |
