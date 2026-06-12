# Prompts para iniciar o projeto PCP

> Sequência de prompts para tocar a reconstrução com o Claude Code, **na ordem de
> dependência**. Cada prompt é autossuficiente. Rode um, valide, só então o próximo.
> Antes de tudo: o Claude Code carrega [CLAUDE.md](CLAUDE.md) automaticamente — ele é o
> contrato. As regras de negócio estão em [docs/prd/](docs/prd/).
>
> Trabalhe sempre **dentro de `PCP/`**. Sugestão: um branch por prompt e commits pequenos.

> **Lembretes transversais (valem para TODOS os prompts — direto do CLAUDE.md):**
> - **100% Rust, nada de Node/npm/JS/Tailwind** (§1). Frontend buildado por
>   `cargo-leptos`/Trunk; estilo em **CSS escrito à mão com design tokens** e escopo por
>   componente via crate Rust (ex.: `stylance`).
> - **UI/UX bonita e moderna** (§16): não basta funcionar. Design system enxuto (tokens +
>   componentes Leptos reutilizáveis), hierarquia visual, micro-interações, responsivo
>   (desktop→mobile) e adaptativo (tema claro/escuro), skeletons por seção.
> - **Fronteira de módulo `pcp`** (§0): o PCP é um módulo do futuro ERP — tabelas com
>   schema/prefixo de domínio `pcp` e rotas sob `/pcp/...`.
> - **Responsabilidade única e arquivos enxutos** (§5/§15): um assunto por arquivo
>   (≤ ~300 linhas, funções ≤ ~50), **um componente Leptos por arquivo**.
> - **Frontend burro em regra** (§3/§13): nunca recalcula status/meta/cobertura/sugestão;
>   só exibe o que a API entrega.
> - **Segurança e auditoria** (§7): deny-by-default; escrita nas tabelas só pelo pipeline;
>   toda escrita de usuário auditada; **secrets só em variáveis de ambiente** (`.env` fora do git).
> - **Definition of Done** (§14) ao fechar cada prompt: compila; **sem `unsafe`**;
>   `cargo fmt` + `cargo clippy -- -D warnings` limpos; regras tocadas têm teste em
>   `pcp-core`; nada hardcoded que devia ser config; migrations versionadas.

---

## Fase 0 — Fundações

### Prompt 0.1 — Esqueleto do workspace + CI
```
Crie o esqueleto do Cargo workspace do projeto PCP conforme a seção 2 do CLAUDE.md.
Gere: Cargo.toml do workspace, rust-toolchain.toml (stable), .gitignore (incluindo .env
e target/), e os crates vazios pcp-core, pcp-config, pcp-db, pcp-engine, pcp-etl, pcp-ai,
pcp-api, pcp-web — cada um com Cargo.toml e um lib.rs/main.rs mínimo que compila.
Cada crate deve declarar no topo `#![forbid(unsafe_code)]` e
`#![warn(clippy::all, clippy::pedantic)]` (CLAUDE.md §5). Configure as dependências entre
crates segundo a "regra de dependência one-way" do CLAUDE.md §2. Crie a pasta tests/ do
workspace (testes de paridade/invariantes — §2/§11). Adicione um README.md curto.
Configure o CI (ex.: GitHub Actions) rodando `cargo fmt --check`, `cargo clippy -- -D
warnings` e os testes do pcp-core EM TODO PUSH (CLAUDE.md §11). Garanta que `cargo build`,
`cargo fmt --check` e `cargo clippy -- -D warnings` passam. Não implemente regra de
negócio ainda. 100% Rust, sem nenhuma dependência fora do ecossistema Rust (§1).
```

### Prompt 0.2 — Configuração de negócio editável
```
Implemente o crate pcp-config: estrutura Rust tipada que carrega e valida o arquivo
config/pcp.config.yaml com TODAS as constantes do CLAUDE.md §12 (= doc 02 §11). Crie o
config/pcp.config.yaml com os valores de referência. Inclua validação (ex.: pareto_a <
pareto_b, clamps coerentes) e testes. Deixe preparado o conceito de "auditoria de
mudança de configuração" (quem/quando/valor anterior) — pode ser só o tipo + trait por
ora. Nenhuma constante de negócio pode estar hardcoded em outro crate.
```

### Prompt 0.3 — Banco, Docker e migrations base
```
Configure o Postgres dedicado via docker-compose.yml (Postgres + volume persistente) e o
setup de migrations com SQLx em migrations/. Use um schema/prefixo de domínio `pcp` para
TODAS as tabelas e mantenha o PCP coeso como módulo do futuro ERP (CLAUDE.md §0). Crie as
migrations das tabelas de ENTRADA conforme doc 05 §2 e doc 04: vendas_dia e
estoque_snapshot, com os índices compostos (codigo_estoque, dt_ref DESC) e as chaves do
CLAUDE.md §6. Deixe o modelo EXTENSÍVEL para a dimensão financeira futura (custo/preço)
sem inviabilizá-la (§6), e já preveja a política de retenção desde o dia 1 (§9) como
metadado/coluna de controle. Implemente no crate pcp-db a conexão (pool), o helper de
migrations e repositórios mínimos de leitura/escrita dessas duas tabelas. Idempotência
por dia. Documente como subir o banco localmente. Sem regra de negócio aqui.
```

### Prompt 0.4 — Autenticação e papéis
```
Implemente autenticação própria no pcp-api (Axum): cadastro/login com senha em argon2,
JWT com expiração curta + refresh, e os papéis analista/gestor/admin (CLAUDE.md §7).
Middleware de autorização deny-by-default: nenhum endpoint de dados sem auth; rotas de
negócio sob o prefixo /pcp/... (§0). Tabelas de usuários (schema pcp) + migration. Testes
de: anônimo não acessa nada; cada papel acessa o que deve. Secrets só via variáveis de
ambiente (.env, fora do git).
```

---

## Fase 1 — Motor PCP (núcleo) — implementar em pcp-core, puro e testado

> Implemente cada módulo **primeiro como função pura em `pcp-core`** (entrada = structs de
> dados, `data_ref`, config; saída = resultado), com testes, **antes** de ligar ao banco.

### Prompt 1.1 — Consolidação de vendas + classificação ABCFDN
```
No pcp-core, implemente (1) a consolidação de vendas por (dt_ref, codigo_estoque) somando
variações LISO+PERSONALIZADO (doc 02 §1) e (2) a classificação A/B/C/D/F/N na ordem de
precedência exata do doc 02 §2 (F→D→N→Pareto, janela ABC 540 dias, D 180d, N 60d, Pareto
A≤80%/B≤95%), lendo limiares de pcp-config. Funções puras, sem I/O. Testes cobrindo cada
classe, a precedência, e os invariantes do doc 08 §4 (nenhum produto com 2 classes; soma
de Pareto = 100).
```

### Prompt 1.2 — Parâmetros de estoque
```
No pcp-core, implemente os parâmetros estatísticos por produto do doc 02 §3: janela 12m só
dias com venda, remoção de outliers IQR 1.5× (só limite superior), média/desvio sem
outliers, coef. de variação, e o ESTOQUE RECOMENDADO UNIFICADO na fórmula meta-ABC do
§3.6 (meta {45,30,15,10,5,20} × fator_sazonal + estoque_seguranca estatístico z=1.28),
com teto de 60 dias de cobertura. Tratamento de produto sem histórico confiável
(< 10 dias com venda) marcando status SEM_HISTORICO_CONFIAVEL com defaults configuráveis.
Funções puras + testes.
```

### Prompt 1.3 — Sazonalidade dinâmica
```
No pcp-core/pcp-engine, implemente a sazonalidade do doc 02 §4: fator por mês = média
diária do mês (ano anterior) / média diária do ano anterior, clamp 0.5–2.0. Gatilho de
recálculo mensal (>30 dias ou mês mudou), failsafe (erro não derruba o pipeline), com log.
Persistência dos 12 fatores. Testes incluindo o clamp e o gatilho.
```

### Prompt 1.4 — Alertas + recomendação de produção
```
No pcp-core, implemente os alertas do doc 02 §6 (prioridade por % do recomendado 20/50/80,
elevação de classe A, ordenação prioridade→classe→qtd) e o serviço ÚNICO de recomendação
de produção do doc 02 §7 unificando §7.1/§7.2/§7.3 (meta-ABC base + fator de urgência +
fator sazonal + proteção de ruptura < 3 dias). qtd_sugerida sobre qtd_disponivel. Campo
prioridade próprio (não reusar configuracao). Status de estoque hierárquico do §5.2.
Testes: produto fora de linha nunca gera alerta; qtd_sugerida ≥ 0; e o invariante de que a
cobertura sentinela 999 nunca entra em médias (§11).
```

### Prompt 1.5 — Ciclo de vida (fora de linha)
```
No pcp-core, implemente a análise de fora de linha do doc 02 §8: pontuação 0–20, decisão
SAIR (≥8) / VOLTAR (≤4 + venda recente), níveis de certeza (≥15/≥10/<10). Modele como UMA
entidade com máquina de estados gerada→em_analise→aplicada/recusada/expirada (não duas
tabelas). Migration (schema pcp) com retenção de 90 dias para não aplicadas. Testes da
pontuação e das transições.
```

### Prompt 1.6 — Motor diário + pipeline idempotente
```
No pcp-engine, implemente o orquestrador diário que roda os 4 módulos na ordem
classificação→parâmetros→alertas→fora de linha (doc 05 §1.2), idempotente por data_ref,
com isolamento de falha por módulo e tabela de execuções (início/fim/duração/linhas/erro,
doc 05 §3). Pré-validação bloqueante (vendas do dia anterior > 0 e snapshot presente).
Comando de reprocesso de data e intervalo. Crie e versione as migrations das tabelas
DERIVADAS (classificação, parâmetros, alertas — schema/prefixo pcp, nomes honestos
CLAUDE.md §3, campo prioridade própria §6) com política de retenção desde já (§9).
Persista os
resultados via pcp-db.
```

### Prompt 1.7 — Importação de dados + teste de paridade
```
No pcp-etl, implemente o ImportadorArquivo (CSV/dump) atrás do trait FonteDados (CLAUDE.md
§1/§8), validando o contrato do doc 05 §2 e gravando vendas_dia/estoque_snapshot de forma
idempotente. Carregue um dump real do legado e rode o motor sobre uma data. Escreva os
testes de paridade do doc 08 §3–4: produtos 6797/10001/10473 e a distribuição esperada
A=165/B=346/C=671/D=1012/F=177/N=9 (tolerância de arredondamento).
```

---

## Fase 2 — Fundação do frontend + telas operacionais (pcp-api + pcp-web/Leptos)

### Prompt 2.1 — API de leitura agregada
```
No pcp-api, exponha os endpoints de leitura (sob /pcp/..., §0) que cobrem as agregações do
doc 04 §6.2 (resumos de estoque, dashboard, produtos detalhados/paginados, alertas
completos, distribuições ABC, cobertura). Materialize/optimize a view de produtos ativos
com refresh pós-pipeline. Todos os valores (status, cobertura, recomendada, sugestão) vêm
já calculados pelo motor — a API NÃO recalcula regra. A cobertura sentinela 999 nunca
entra em médias (§11). Paginação no servidor. Exponha também um canal SSE (CLAUDE.md §16)
que notifica fim de processamento / mudança de estado para o frontend assinar. Um
handler por arquivo, responsabilidade única (CLAUDE.md §15). Testes de contrato dos payloads.
```

### Prompt 2.2 — Fundação do frontend: design system, shell, login e tema (Leptos)
```
No pcp-web (Leptos, SSR + hidratação WASM), monte a FUNDAÇÃO do frontend, 100% Rust e sem
Node (CLAUDE.md §1): configure o build por cargo-leptos/Trunk. Crie um DESIGN SYSTEM
enxuto e BONITO/MODERNO (CLAUDE.md §16): arquivo de design tokens (paleta, tipografia,
espaçamento, raios, sombras, elevação), tema claro/escuro (respeitando prefers-color-scheme),
e componentes Leptos reutilizáveis (botão, card, tabela, badge, input, modal, skeleton),
um componente por arquivo (§15). Estilo em CSS escrito à mão com escopo por componente via
crate Rust (ex.: stylance) — sem Tailwind/JS. Implemente o shell da aplicação: página de
LOGIN (consumindo a auth do 0.4), layout autenticado (sidebar + header), navegação do mapa
do doc 03, e o gate de autenticação. Tudo responsivo (desktop→mobile) e acessível
(contraste, foco visível, navegação por teclado). Cores semânticas consistentes (semáforo
de status; cor fixa por classe ABC — §12). Sem regra de negócio no frontend.
```

### Prompt 2.3 — Central de Alertas (Leptos)
```
No pcp-web, implemente a Central de Alertas do doc 03 §5 sobre o design system do 2.2: fila
do dia ordenada por urgência, cards de resumo, flag de ruptura iminente, filtros, link para
detalhe. Frontend burro: consome a API, não recalcula nada. Atualização em TEMPO REAL via
SSE (CLAUDE.md §16): a tela reflete novo processamento/alerta sem reload, com fallback de
polling. UI bonita e moderna (§16), responsiva (desktop→mobile) e adaptativa (tema
claro/escuro). Um componente Leptos por arquivo, responsabilidade única (§15).
Nomenclatura do CLAUDE.md §12.
```

### Prompt 2.4 — Gestão de Estoque (Leptos)
```
Implemente a página de Gestão de Estoque do doc 03 §3 sobre o design system do 2.2: tabela
paginada no servidor, cards clicáveis que aplicam filtro, filtros avançados (busca, classe,
status, faixas, switches), ordenação, filtros salvos nomeados (persistidos por usuário no
backend), e exportação CSV UTF-8 com BOM / JSON do FILTRO COMPLETO. Colunas do §3.3
(incluindo "Recomendada" — §12). UI bonita/moderna, responsiva e adaptativa (§16). Frontend
burro: não recalcula nada.
```

### Prompt 2.5 — Detalhe do Produto + Solicitação de Produção real
```
Implemente o Detalhe do Produto do doc 03 §4 sobre o design system do 2.2 (cabeçalho com
regra da classe, métricas, gráficos 90d de vendas e estoque — gráficos em Rust). Implemente
a Solicitação de Produção REAL (não simulada): persiste produto/qtd/prioridade/prazo/
solicitante/justificativa, com máquina de estados pendente→aprovada→em produção→concluída e
AUDITORIA (§7.5). Cálculo pelo serviço único de recomendação (doc 02 §7), vindo da API.
UI bonita/moderna, responsiva e adaptativa (§16).
```

---

## Fase 3 — Dashboard + ABC + Configurações + workflow de aprovação

### Prompt 3.1 — Dashboard executivo
```
Implemente o Dashboard do doc 03 §2 sobre o design system do 2.2, com carregamento
progressivo por seção (skeletons): gráfico de estoque 30 dias por classe, painel de metas
físicas ABC (50/30/20/0, ±3 p.p.), cards (produtos, críticos, cobertura, cobertura por
classe), top ABC com lazy load, alertas recentes, distribuição ABC. Cores de criticidade
do doc 02 §9.2. Tudo vindo da API (frontend burro). UI bonita/moderna, responsiva e
adaptativa (§16).
```

### Prompt 3.2 — Classificação ABC + workflow de fora de linha
```
Implemente a página ABC do doc 03 §6 sobre o design system do 2.2 (Pareto top 20, tabela 1
linha por produto com a classificação mais recente, exportação) e o workflow de fora de
linha: fila de sugestões do dia com nível de certeza, aprovação pelo gestor com trilha de
auditoria (§7.5), aplicação que muda o estado da entidade de ciclo de vida. UI
bonita/moderna, responsiva e adaptativa (§16).
```

### Prompt 3.3 — Tela de Configurações (parâmetros editáveis + usuários)
```
Implemente a tela de Configurações do doc 03 §8 (promovida a requisito — CLAUDE.md §12/§13)
sobre o design system do 2.2: (1) edição de TODAS as constantes de negócio do doc 02 §11
com trilha de auditoria (quem mudou, quando, valor anterior — §7.5), persistidas via
pcp-config; (2) fatores sazonais (visualizar vigentes/histórico/desvio real×previsto e
override manual com justificativa); (3) gestão de usuários e papéis analista/gestor/admin;
(4) preferências de exibição por usuário (colunas, página inicial, tamanho de página).
Autorização por papel (gestor edita config; admin gere usuários — §7.3). Toda escrita
auditada; frontend burro consumindo a API. UI bonita/moderna, responsiva e adaptativa (§16).
```

---

## Fase 4 — IA (pcp-ai, Anthropic Claude)

### Prompt 4.1 — Insights estatísticos no backend
```
No pcp-ai, implemente os insights estatísticos do doc 06 §3 no backend (regressão linear,
médias móveis 7d, suavização exponencial, decomposição sazonal por dia da semana, IQR,
previsão 7/30d, alertas inteligentes do §3.3) com cache por produto/dia. Alimenta a página
de produto e os alertas inteligentes. Sem mocks (cortar correlações aleatórias do legado).
```

### Prompt 4.2 — Chat IA com tool use (Claude)
```
No pcp-ai, implemente o Chat IA do doc 06 §1 com Anthropic Claude e tool use sobre dados
reais (ferramentas do §1.3, SOMENTE LEITURA, limites de linha). Consulte a skill
claude-api para IDs de modelo, params e formato de tool use. Modelo/provider configuráveis;
chave do Claude SOMENTE em variável de ambiente / secret manager (§7.4/§10). Histórico
persistido por usuário com autorização (ferramenta, args, tokens, tempo). Sugestões de
próximo passo, perguntas rápidas, transparência da consulta. Telemetria de tokens/custo,
timeout, retry com backoff. A UI do chat (doc 03 §7) usa o design system do 2.2 (§16).
```

### Prompt 4.3 — Análise por produto via LLM
```
Implemente a análise por produto do doc 06 §2 com Claude, contrato JSON fixo do §2.3 e
FALLBACK LOCAL obrigatório (motor doc 02 §7) se a chamada falhar. Provider/modelo
configuráveis; chave somente em env/secret manager (§7.4/§10). Registrar custo/tokens.
```

---

## Fase 5 — Operação e cut-over
```
Implemente: painel admin de status do pipeline + reprocesso de data/intervalo;
notificação de falha (webhook); rotinas de expurgo/retenção completas (CLAUDE.md §9);
health checks do doc 05 §4; deploy em VPS via Docker com ambientes separados (dev/staging/
prod) e migrations aplicadas no CI (o CI de fmt/clippy/testes já existe desde o 0.1).
Prepare o período de operação paralela com comparação diária automática contra o legado
(doc 09 Fase 5).
```

---

### Backlog pós-MVP (não começar antes da hora)
Dimensão financeira (custo/preço → capital parado, ROI) · alertas preditivos 7/15/30d +
notificações proativas classe A · índice de saúde do estoque 0–100 · integração com
agendamento de produção · multi-tenant · primeiros módulos do "gestor empresarial".
