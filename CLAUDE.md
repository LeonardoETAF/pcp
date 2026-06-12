# Regras Fixas — Projeto PCP

> **Este arquivo é o contrato de desenvolvimento.** Toda decisão, código e PR devem
> respeitá-lo. Está nomeado `CLAUDE.md` para ser carregado automaticamente a cada
> sessão do Claude Code. As regras de **negócio** canônicas estão em
> [docs/prd/02-regras-de-negocio.md](docs/prd/02-regras-de-negocio.md) — quando este
> arquivo e o PRD divergirem em regra de negócio, **vale o PRD §02**.

---

## Princípios mestres (leia primeiro)

Tudo neste projeto serve a dois objetivos acima de qualquer outro: **DESEMPENHO** e
**SEGURANÇA**. Em toda decisão, prefira a opção que os maximiza sem sacrificar clareza.

1. **Rust idiomático.** Trabalhe *com* o borrow checker, não contra ele. Ownership e
   lifetimes claros; `&T`/`&mut T` em vez de clones desnecessários; iteradores em vez de
   loops manuais; `Option`/`Result` em vez de sentinelas; pattern matching exaustivo.
2. **Zero-cost abstractions.** Abstraia por traits/generics/newtypes que somem na
   compilação — nunca por indireção que custa em runtime. Sem alocação onde não precisa.
3. **Gerência de memória segura — SEM `unsafe`.** Todo crate declara
   `#![forbid(unsafe_code)]`. Nenhuma exceção sem decisão explícita e documentada do dono.
4. **Concorrência otimizada e correta.** Async/Tokio no I/O; paralelismo (ex.: rayon) só
   onde mede ganho. Estado compartilhado preferencialmente imutável; quando mutável,
   `Arc`/tipos do tokio com escopo mínimo. Sem locks no caminho quente sem necessidade.
5. **Modularidade real (backend e frontend).** O sistema é pensado **por módulos**: o
   **PCP é apenas um módulo do futuro ERP**. Fronteiras explícitas, baixo acoplamento,
   alta coesão. Um módulo não conhece as entranhas de outro — só seu contrato.
6. **Responsabilidade única.** Cada arquivo, cada `struct`/módulo e cada função fazem
   **uma coisa**. Arquivos pequenos e focados (ver §15). Se algo cresce demais ou faz
   "e também...", divida.
7. **Tempo real, responsivo e adaptativo.** A UI reflete mudanças sem reload (§16),
   funciona de desktop a mobile (responsiva) e se adapta a tema/densidade/preferências.

Detalhamento operacional destes princípios em **§5**, **§15** e **§16**.

---

## 0. O que é o projeto

- **PCP** (Planejamento e Controle de Produção) de uma fábrica de copos/plásticos
  personalizáveis (~90% produção própria). Reconstrução do legado documentado em
  [docs/prd/](docs/prd/), **preservando as regras de negócio validadas** e corrigindo
  os débitos arquiteturais do doc 08.
- **Visão de longo prazo:** virar um **gestor empresarial (ERP) completo**. Pense o
  sistema **por módulos**: **o PCP é um módulo do futuro ERP** — não "o sistema". Cada
  capacidade futura (financeiro, vendas, compras, estoque comercial, RH...) entrará como
  **novo módulo/crate independente**, plugando-se a um núcleo comum (auth, config,
  usuários, infra de dados) **sem reescrever nem acoplar** o que já existe.
- **Contrato entre módulos:** cada módulo expõe uma API/serviço bem definido e consome os
  outros **apenas por esse contrato**. Nada de um módulo alcançar tabelas ou tipos
  internos de outro. O PCP deve ser construído já respeitando essa fronteira (ex.: nome
  de domínio/prefixo `pcp` nas tabelas, rotas sob `/pcp/...`).
- **Agora o escopo é só PCP.** Não construir módulos do futuro ERP antes da hora — mas não
  tomar decisão que os inviabilize.
- **Single-tenant** (apenas SuperCopo) por enquanto, mas **sem premissas que impeçam
  multi-tenant depois** (ex.: não espalhar identificadores globais que dificultem um
  futuro `tenant_id`).
- Todo o projeto vive **dentro da pasta `PCP/`**.

## 1. Stack — FIXA, não renegociar sem decisão explícita do dono

| Camada | Decisão |
|---|---|
| Linguagem | **Rust** em backend **e** frontend (100% Rust) |
| Frontend | **Leptos** (SSR + hidratação WASM) |
| Backend HTTP | **Axum** (async, tower) |
| Acesso a dados | **SQLx** (SQL escrito à mão, verificado em compile-time) |
| Banco | **PostgreSQL dedicado, self-hosted via Docker** |
| LLM | **Anthropic Claude** (tool use / function calling) |
| ETL | **Nativo em Rust** (crate `pcp-etl`) |
| Auth | **Próprio: JWT + papéis** `analista` / `gestor` / `admin` |
| Deploy | **VPS Linux**, via Docker / docker-compose |
| Idioma do produto | **pt-BR** (UI, mensagens, datas, números) |

- **Edição Rust 2021+**, toolchain estável fixada em `rust-toolchain.toml`.
- **100% Rust, nada fora do ecossistema Rust.** Sem Node/npm, sem JS frameworks, sem
  Tailwind ou qualquer ferramenta que exija toolchain externa. Build do frontend por
  **`cargo-leptos`/Trunk**. O estilo é **CSS escrito à mão** (asset da plataforma web, não
  ferramenta externa) com **design tokens** e escopo por componente via crate Rust
  (ex.: `stylance`). Ver §16.
- Async runtime: **Tokio**. Erros: **`thiserror`** em libs, **`anyhow`** só nas bordas
  (bins). Logging: **`tracing`** estruturado. Datas: **`chrono`** (datas de negócio são
  `date`, fuso América/São_Paulo na exibição). Serialização: **`serde`**.
- O ERP **"One" ainda não tem API**. Então o ETL inicia por **importação de arquivo
  (CSV/dump)** seguindo o contrato do doc 05 §2; o conector direto ao ERP é tarefa
  posterior, atrás de um **trait** (`FonteDados`) para não acoplar o resto ao ERP.

## 2. Estrutura do workspace (Cargo)

```
PCP/
├── Cargo.toml              # workspace
├── rust-toolchain.toml
├── CLAUDE.md               # este arquivo
├── docs/prd/               # PRD canônico (contrato de negócio e funcional)
├── config/
│   └── pcp.config.yaml     # constantes de negócio EDITÁVEIS (doc 02 §11)
├── migrations/             # TODAS as migrations versionadas (nunca só no banco)
├── docker-compose.yml      # Postgres + app
├── crates/
│   ├── pcp-core/   # DOMÍNIO PURO: todas as regras do doc 02. Sem I/O. 100% testável.
│   ├── pcp-config/ # carrega/valida pcp.config.yaml + auditoria de mudanças
│   ├── pcp-db/     # repositórios SQLx, modelos de persistência, migrations helper
│   ├── pcp-engine/ # motor diário: orquestra os 4 módulos sobre pcp-core + pcp-db
│   ├── pcp-etl/    # ingestão (arquivo/CSV agora; conector ERP "One" depois)
│   ├── pcp-ai/     # chat IA, análise por produto, insights estatísticos (Claude)
│   ├── pcp-api/    # servidor Axum: auth, autorização, endpoints de leitura/escrita
│   └── pcp-web/    # frontend Leptos
└── tests/          # testes de paridade e invariantes (regressão de regra)
```

**Regra de dependência (one-way, núcleo no centro):**
`pcp-core` não depende de nada do projeto. `pcp-engine`, `pcp-ai`, `pcp-api` dependem de
`pcp-core`/`pcp-db`/`pcp-config`. **`pcp-web` nunca importa regra de negócio** — só
consome valores prontos da API.

## 3. Princípios arquiteturais inegociáveis (do doc 01 §7 e doc 08)

1. **Motor de cálculo único e versionado.** Cada regra de negócio existe **uma só vez**,
   em `pcp-core`. Proibido duplicar regra em SQL, na API e no frontend (foi o pior débito
   do legado).
2. **Frontend burro em regra.** `pcp-web` exibe o que a API entrega. Não recalcula
   status, metas, cobertura, sugestão — nada. Se precisar de um número, a API o fornece.
3. **Pipeline idempotente.** Reprocessar uma `data_ref` substitui os resultados daquela
   data sem efeito colateral (delete+insert ou upsert por chave do dia).
4. **Ordem do motor importa:** classificação → parâmetros → alertas → fora de linha
   (cada etapa alimenta a seguinte). Isolamento de falha por módulo + telemetria.
5. **Retenção definida desde o dia 1** (ver §9). Nunca acumular tabela sem expurgo.
6. **Segurança por padrão (deny-by-default).** Ver §7.
7. **Configuração editável, nada hardcoded.** Todas as constantes do doc 02 §11 vivem em
   `config/pcp.config.yaml`, lidas por `pcp-config`, com auditoria de mudança.
8. **Nomes honestos.** Sem repetir mentiras do legado (`volume_12m` que guardava 18m,
   `estoque_min_15d` que não era 15 dias). Nome reflete o conteúdo; janela em metadado.

## 4. Regras de negócio — o PRD §02 é o contrato

- **Fonte da verdade:** [docs/prd/02-regras-de-negocio.md](docs/prd/02-regras-de-negocio.md).
  Implementar exatamente: classificação A/B/C/D/F/N (precedência F→D→N→Pareto, janela ABC
  18m), parâmetros estatísticos (12m, IQR 1.5×, z=1.28, teto 60d), **estoque recomendado
  unificado na fórmula meta-ABC** {A45/B30/C15/D10/F5/N20} × sazonal + segurança
  estatística, sazonalidade (clamp 0.5–2.0, auto-update mensal), alertas (% do recomendado
  20/50/80 + elevação classe A), status de estoque hierárquico (criticidade A≤15/B≤10/C≤5),
  ciclo de vida (pontuação 0–20).
- **As constantes vêm de `config/pcp.config.yaml`**, nunca de literais no código. O bloco
  YAML de referência está no doc 02 §11 e reproduzido em §12 deste arquivo.
- **Decisões de unificação já tomadas** (não reabrir): fatores ABC 1.2/1.0/0.8/0.3/0.1/0.8;
  janela ABC 18 meses; recomendado = meta-ABC (não a base 15d); z=1.28; status canônico
  hierárquico; metas {45,30,15,10,5,20}; alertas por % do recomendado; reposição sobre
  `qtd_disponivel`. (doc 08 §1.)

## 5. Convenções de código

- **Todo crate** declara no topo do `lib.rs`/`main.rs`:
  `#![forbid(unsafe_code)]` e `#![warn(clippy::all, clippy::pedantic)]`.
  **Nenhum `unsafe`** no projeto (Princípio mestre 3).
- `cargo fmt` e `cargo clippy -- -D warnings` **passam antes de todo commit**.
- **Responsabilidade única e arquivos enxutos:** um arquivo = um assunto. Alvo prático:
  **≤ ~300 linhas** por arquivo `.rs` e **≤ ~50 linhas** por função (referência, não
  dogma cego — mas estourar muito é sinal para dividir). Componente Leptos idem: um
  componente por arquivo, com uma responsabilidade. Ver §15.
- **Idiomático e zero-cost:** preferir `&T`/slices a clones; iteradores/`?`/combinadores a
  loops e `match` verbosos; `newtype` para tipos de domínio; generics/traits a `dyn` no
  caminho quente. Sem `clone()` por preguiça, sem alocação evitável em loop.
- Funções de regra em `pcp-core` são **puras** (entrada → saída, sem relógio/banco/rede).
  O "tempo" entra como parâmetro (`data_ref: NaiveDate`), nunca `now()` dentro da regra.
- **Aritmética de negócio explícita:** arredondamento conforme o PRD (`CEIL`, `ROUND`),
  documentado por função. Quantidades são inteiras (unidades). Evitar `f64` onde a regra
  pede inteiro; usar tipos próprios (`newtype`) para `CodigoEstoque`, `ClasseAbc`, etc.
- Erros tipados (`thiserror`); nunca `unwrap()`/`panic!` em caminho de produção (testes ok).
- Comentários só onde a regra não é óbvia — e citando o §do PRD (`// doc 02 §3.5`).
- Nada de `TODO` sem dono/data. Nada de mock que finja dado real (o legado tinha
  correlações aleatórias e `setTimeout` simulando ação — proibido).

## 6. Banco de dados

- **Postgres dedicado**, só do PCP (não compartilhar com outros sistemas — doc 07 §4).
- **Todas** as migrations no repositório (`migrations/`), versionadas e aplicadas por
  `sqlx migrate`. Proibido alterar schema só no banco.
- Tabelas de entrada: `vendas_dia`, `estoque_snapshot` (contrato doc 05 §2). Múltiplas
  linhas por (dia, código) em vendas; snapshot **completo** por dia. Idempotência por dia.
- Tabelas derivadas escritas **só pelo motor**. Nomes honestos, `prioridade` própria no
  alerta (não reusar `configuracao`).
- Índices compostos `(codigo_estoque, dt_ref DESC)` nas históricas. Materializar a view
  principal de produtos ativos com refresh pós-pipeline.
- Dimensão financeira (custo/preço) **adiada** — mas deixar o modelo extensível (não
  inviabilizar a coluna depois).

## 7. Segurança (prioridade máxima — maior débito do legado)

1. **Deny-by-default:** nenhum dado de negócio sem autenticação. Sem endpoint público de
   dados. Autorização verificada na `pcp-api` por papel.
2. **Escrita nas tabelas de entrada/derivadas só pelo pipeline** (credencial de serviço
   própria). Usuário final **só lê**; suas escritas são solicitações/aplicações auditadas.
3. **Papéis:** `analista` (lê tudo, cria solicitações); `gestor` (+ aprova fora de
   linha/solicitações, edita configurações); `admin` (+ gestão de usuários).
4. **Secrets** (Claude API key, banco, ERP futuro) **só em variáveis de ambiente / secret
   manager**. NUNCA em código, README, frontend ou git. `.env` no `.gitignore`.
5. **Auditoria** de toda escrita de usuário (aplicar fora de linha, editar config, criar
   solicitação): quem, quando, valor anterior.
6. **Chat IA é somente-leitura** com allowlist de consultas (doc 06 §1.6).
7. Senhas com **argon2**; tokens JWT com expiração curta + refresh.

## 8. ETL e pipeline (doc 05)

- Contrato de dados do doc 05 §2 é obrigatório. **Pré-validação bloqueante:** não
  processa sem vendas do dia anterior > 0 e snapshot do dia presente (tolerância
  configurável + notificação).
- **Idempotência** por data; reprocesso de data/intervalo via admin.
- **Tabela de execuções** do pipeline (por módulo: início, fim, duração, linhas, erro) —
  visível na UI admin. Falha de módulo → notificação + banner "dados de DD/MM".
- Fonte de dados atrás do trait `FonteDados`: hoje `ImportadorArquivo` (CSV/dump);
  amanhã `ErpOne` quando houver API. O motor não sabe de onde vêm os dados.
- SLA: dados do dia prontos até **05:00**.

## 9. Retenção de dados (doc 07 §6) — implementar expurgo desde o início

| Dado | Retenção |
|---|---|
| `vendas_dia` | permanente (base dos cálculos) |
| `estoque_snapshot` | ≥ 24 meses |
| classificação diária | ≥ 24 meses |
| alertas | 12 meses |
| sugestões de ciclo de vida não aplicadas | **90 dias** (legado acumulou 33,7M de linhas) |
| logs de pipeline | 12 meses |
| conversas de chat | indefinida, exclusão sob demanda do usuário |

- Ciclo de vida = **uma** entidade com máquina de estados
  (`gerada → em_analise → aplicada/recusada/expirada`), não duas tabelas como no legado.

## 10. IA (doc 06) — Anthropic Claude

- **Chat IA** com tool use sobre dados reais (nunca inventa número). Ferramentas do doc 06
  §1.3, **somente leitura**, com limites de linhas (50–100). Histórico persistido por
  usuário (com autorização), incluindo ferramenta chamada, args, tokens, tempo.
- **Análise por produto** com contrato JSON fixo do doc 06 §2.3 e **fallback local
  obrigatório** (motor de §7 do doc 02) se a chamada LLM falhar.
- **Insights estatísticos no backend** (`pcp-ai`), não no frontend: regressão linear,
  médias móveis, IQR, decomposição sazonal, previsão 7/30d (doc 06 §3).
- **Modelo/provider configuráveis** (não hardcoded). Registrar tokens/custo, timeout,
  retry com backoff. Usar modelos Claude atuais (ex.: Opus 4.8 / Sonnet 4.6) — ao mexer
  na integração LLM, **consultar a skill `claude-api`** para IDs, params e tool use.

## 11. Testes obrigatórios (regressão de regra — doc 08 §3–4)

- **Produtos de referência** `6797`, `10001`, `10473`: reproduzir os cálculos e comparar
  com o legado antes de qualquer cut-over.
- **Distribuição esperada** (jun/2026) como teste de aceitação aproximado:
  A=165 · B=346 · C=671 · D=1.012 · F=177 · N=9.
- **Invariantes** (testes de propriedade): soma dos % de Pareto = 100; nenhum produto com
  2 classes no mesmo dia; `qtd_sugerida ≥ 0`; produto fora de linha nunca gera alerta;
  cobertura sentinela 999 nunca entra em médias.
- CI roda `fmt` + `clippy -D warnings` + testes do `pcp-core` em todo push.

## 12. Nomenclatura e UX (doc 02 §10, doc 03 §10) — obrigatórias

- **"Produzir" / "Produção"** — nunca "Comprar".
- Coluna de alvo de estoque chama-se **"Recomendada"** (exibe `estoque_total_recomendado`).
- Variação exibida como `"{produto} - {valor após ':' em configuracao}"`.
- Cobertura `999` exibe **"Sem histórico"**, nunca o número.
- Datas em formato BR; quantidades com separador de milhar. Tema claro/escuro.
- Cores semânticas consistentes (semáforo de status; cor fixa por classe ABC).
- Carregamento progressivo por seção (skeletons); nenhuma tela "toda branca".
- Exportações: CSV UTF-8 **com BOM** (Excel BR) e JSON; exportar o **filtro completo**,
  não só a página.

## 13. Proibido (lições do doc 08 — não repetir o legado)

- ❌ Regra de negócio duplicada fora do `pcp-core`.
- ❌ Frontend recalculando status/meta/cobertura.
- ❌ Constante de negócio hardcoded (vai em `pcp.config.yaml`).
- ❌ Tabela sem política de retenção.
- ❌ Tabela de dados sem autorização; secret em código/README/frontend.
- ❌ Migration que existe só no banco.
- ❌ Mock que finge dado real; ação "simulada" (setTimeout, localStorage como backend).
- ❌ Prioridade de alerta gravada em coluna de outro propósito.
- ❌ Banco compartilhado com outros sistemas.
- ❌ Múltiplas versões da mesma função/conceito convivendo. Uma só, sempre.

## 14. Definition of Done (por entrega)

Uma tarefa só está pronta quando: compila; **sem `unsafe`**; `fmt` + `clippy -D warnings`
limpos; regras tocadas têm teste em `pcp-core`; nada hardcoded que devia ser config; sem
regra duplicada no frontend; **arquivos e funções com responsabilidade única e tamanho
sob controle (§15)**; **fronteiras de módulo respeitadas (§0)**; migrations versionadas;
secrets fora do código; documentado o §do PRD que a tarefa implementa.

## 15. Modularidade, estrutura e desempenho (detalhe dos Princípios mestres 1–6)

**Organização física do código (backend e frontend):**
- Um arquivo por assunto, **uma responsabilidade por arquivo**. Alvo ≤ ~300 linhas/arquivo,
  ≤ ~50 linhas/função (referência). `lib.rs`/`mod.rs` só declaram e reexportam — **não**
  concentram lógica.
- Dentro de cada crate, separar por responsabilidade, não por "tudo num arquivo". Ex.:
  `pcp-core/src/classificacao/{mod.rs, pareto.rs, precedencia.rs}`,
  `pcp-api/src/rotas/{estoque.rs, alertas.rs, ...}`. Um endpoint/handler por arquivo
  quando crescer. No `pcp-web`, **um componente Leptos por arquivo**, agrupados por
  feature/página (`web/src/paginas/`, `web/src/componentes/`).
- **Fronteira de módulo de negócio:** o PCP é um módulo do ERP (§0). Mantenha tudo do PCP
  coeso (tabelas com prefixo/domínio `pcp`, rotas `/pcp/...`, tipos no namespace do PCP)
  para que um próximo módulo (ex.: `financeiro`) nasça ao lado sem tocar no PCP. Núcleo
  comum (auth, usuários, config, infra de db) fica em crates compartilháveis.

**Desempenho (medir, não chutar):**
- Agregações pesadas **no banco** (SQL/materialized view), nunca montadas no cliente.
- Paginação no servidor; índices compostos `(codigo_estoque, dt_ref DESC)`.
- Caches de leitura com `staleTime` por volatilidade; refresh da view principal pós-pipeline.
- Concorrência: I/O em async; só paralelizar CPU (rayon) onde houver ganho medido. Evitar
  contenção de lock no caminho quente; preferir dados imutáveis compartilhados via `Arc`.
- Antes de "otimizar" algo não-óbvio, **meça** (bench/criterion ou `tracing` com tempos).
  Clareza primeiro; micro-otimização só com número que a justifique.

## 16. Estética, tempo real, responsividade e adaptatividade (Princípio mestre 7)

- **UI/UX bonita e moderna (requisito, não enfeite):** o produto deve ter design de
  qualidade — não basta funcionar. Construir um **design system enxuto em Rust**: arquivo
  de **design tokens** (paleta, tipografia, espaçamento, raios, sombras, elevação),
  componentes Leptos reutilizáveis (botões, cards, tabelas, badges, inputs, modais),
  hierarquia visual clara, micro-interações e transições sóbrias, estados vazios/erro/loading
  cuidados. **Estilo 100% CSS à mão** (sem Node/Tailwind — ver §1), escopado por componente
  via crate Rust (ex.: `stylance`), empacotado pelo `cargo-leptos`. Cores semânticas
  consistentes (semáforo de status; cor fixa por classe ABC — §12). Acessibilidade básica
  (contraste, foco visível, navegação por teclado, `aria` onde couber).
- **Atualizações em tempo real:** telas que refletem o pipeline (Central de Alertas,
  Dashboard, status do pipeline) atualizam **sem reload**. Implementar via **SSE**
  (server-sent events) por padrão — ou WebSocket onde houver comunicação bidirecional —
  no `pcp-api`, com o `pcp-web` (Leptos) assinando e invalidando o cache da seção afetada.
  Gatilho: fim do processamento diário, nova solicitação de produção, mudança de estado.
  Sempre com fallback de polling leve se a conexão cair.
- **Responsividade:** desktop-first, **funcional em tablet e mobile**. Layout fluido, sem
  quebra; tabelas grandes com estratégia mobile (scroll/colapso de colunas).
- **Adaptatividade:** tema claro/escuro; respeitar preferências do usuário (colunas
  visíveis, página inicial, tamanho de página — doc 03 §8) e do sistema (prefers-color-scheme).
- **Desempenho percebido:** carregamento progressivo por seção com skeletons; nenhuma tela
  "toda branca" esperando uma query (doc 03 §10).

---

### Constantes de negócio (referência — fonte executável: `config/pcp.config.yaml`)

```yaml
classificacao:
  janela_abc_dias: 540          # 18 meses
  janela_classe_d_dias: 180     # 6 meses sem vendas → D
  janela_produto_novo_dias: 60  # 1ª venda < 60 dias → N
  pareto_a: 80
  pareto_b: 95
  fator_estoque: {A: 1.20, B: 1.00, C: 0.80, D: 0.30, F: 0.10, N: 0.80}
metas_cobertura_dias: {A: 45, B: 30, C: 15, D: 10, F: 5, N: 20, default: 15}
limiar_critico_dias: {A: 15, B: 10, C: 5}
parametros_estoque:
  janela_vendas_meses: 12
  min_dias_com_vendas: 10
  outlier_iqr_mult: 1.5
  z_score_seguranca: 1.28       # 90% nível de serviço
  dias_base_minimo: 15
  teto_cobertura_dias: 60
  defaults_sem_historico: {media: 50, min: 750, seguranca: 250, recomendado_max: 1000}
sazonalidade: {clamp_min: 0.5, clamp_max: 2.0, atualizar_apos_dias: 30}
alertas: {critico_pct: 0.20, alto_pct: 0.50, medio_pct: 0.80, elevar_classe_a: true}
reposicao:
  fator_urgencia: {cobertura_lt_7: 1.5, cobertura_lt_15: 1.2, default: 1.0}
  protecao_ruptura_dias: 3
  aprovacao_automatica: {qtd_max: 1000, exceto_prioridade: alta}
  lead_time_dias: {alta: 7, media: 10, baixa: 15}
fora_de_linha: {limiar_sugerir_saida: 8, limiar_sugerir_volta: 4, alta_certeza: 15, media_certeza: 10}
metas_estoque_fisico_pct: {A: 50, B: 30, C: 20, D: 0}
```
