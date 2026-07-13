# 07 — Requisitos Não Funcionais

## 1. Segurança (prioridade máxima — principal débito do legado)

### 1.1 Estado atual (para entender o risco)
- **Todas as tabelas PCP estão sem RLS/autorização**: qualquer pessoa com a chave anônima (que está no bundle do frontend e até em README) consegue **ler e escrever** em `vendas_dia`, `estoque_snapshot`, `alerta_producao`, `classificacao_abc`, `estoque_param_v2` etc.
- Apenas as tabelas de chat têm RLS (`auth.uid()`).
- Chaves expostas em documentação versionada.

### 1.2 Requisitos do novo sistema
| # | Requisito |
|---|---|
| 1 | Toda tabela com política de acesso desde a criação (deny-by-default) |
| 2 | Escrita nas tabelas de entrada/derivadas **somente** pelo pipeline (credencial de serviço); usuários finais só leem |
| 3 | Autenticação obrigatória para qualquer dado de negócio; sem endpoints públicos de dados |
| 4 | Papéis: `analista` (ler tudo, criar solicitações), `gestor` (analista + aprovar fora de linha/solicitações + editar configurações), `admin` (gestor + usuários) |
| 5 | Secrets (OpenAI, ERP, banco) em secret manager; nunca em código, README ou frontend |
| 6 | Auditoria de ações de escrita do usuário (aplicar fora de linha, editar configuração, criar solicitação): quem, quando, valor anterior |
| 7 | Chat IA: acesso somente leitura, com allowlist de consultas (doc 06 §1.6) |

## 2. Performance

| Métrica | Alvo (baseado no comportamento atual) |
|---|---|
| Dashboard: primeira seção visível | < 1 s |
| Dashboard completo | < 3 s |
| Tabela de estoque (50 itens + filtros) | < 1 s |
| Detalhe do produto (com histórico 90d) | < 2 s |
| Pipeline diário completo (~2.500 produtos) | < 60 s (atual: 2–5 s nos módulos SQL) |
| Resposta do chat IA | < 15 s por turno |

Estratégias validadas no legado (manter):
- Agregações pré-calculadas no banco (a péssima alternativa — agregar no cliente — chegou a ser usada e foi removida);
- Carregamento progressivo por seção + lazy load com intersection observer;
- Cache de leitura com staleTime de 2–10 min conforme volatilidade do dado;
- Fallback em cascata para consultas pesadas (versão "rápida" da agregação);
- Materialização da view principal com refresh pós-pipeline;
- Índices compostos `(codigo_estoque, data DESC)` nas tabelas históricas.

## 3. Disponibilidade e operação

| Requisito | Detalhe |
|---|---|
| SLA dos dados | Dados do dia prontos até **05:00** (doc 05) |
| Janela de uso crítico | 05:00–18:00 dias úteis |
| Falha do pipeline | Notificação imediata + dashboard exibe banner "dados de DD/MM" com data do último processamento bem-sucedido |
| Reprocessamento | Reprocessar qualquer data ou intervalo via admin (idempotente) |
| Backup | Diário, com teste de restauração; retenção ≥ 30 dias |

## 4. Isolamento do ambiente (lição do legado)

O banco atual é **compartilhado com ~10 outros sistemas** da empresa (CRM, SAC, logística, artes, marketing...), com dezenas de edge functions e cron jobs alheios ao PCP. Consequências: risco de contenção, blast radius de incidentes, dificuldade de gestão de acesso.

**Requisito:** o novo PCP roda em **projeto/banco dedicado** (ou, no mínimo, schema isolado com credenciais próprias e quotas separadas).

## 5. Observabilidade

1. Log estruturado de cada execução do pipeline (por módulo: duração, linhas afetadas, erro) — consultável na UI admin.
2. Métricas de uso da IA: tokens, custo estimado, taxa de erro por dia.
3. Health checks do doc 05 §4 expostos em painel de status.
4. Erros de frontend reportados (ex.: Sentry ou equivalente).

## 6. Retenção de dados

| Dado | Retenção sugerida |
|---|---|
| `vendas_dia` | permanente (base dos cálculos — 18m+ necessários) |
| Snapshots de estoque | ≥ 24 meses (gráficos e auditoria) |
| Classificação diária | ≥ 24 meses (análise de migração de classe) |
| Alertas | 12 meses |
| Análises de fora de linha não aplicadas | 90 dias (**o legado acumulou 33,7M de linhas sem expurgo**) |
| Logs de pipeline | 12 meses |
| Conversas de chat | indefinida (por usuário, com exclusão sob demanda) |

## 7. Compatibilidade e implantação

- Aplicação web (desktop-first, responsiva).
- Browsers evergreen (Chrome/Edge/Firefox/Safari atuais).
- Deploy com ambientes separados (dev/staging/prod) e migrations versionadas no repositório — **todas** (no legado, 20+ migrations existem só no banco remoto, irrecuperáveis do código).
- CI com testes das regras de negócio (ver doc 08 §4 — produtos de referência para regressão).
