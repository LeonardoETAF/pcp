# `public/images/` — logo e imagens de conteúdo

Servidas em `/images/<arquivo>`. Todas em **SVG vetorial** (export CorelDRAW). Estado atual:

| Arquivo                 | Cor                    | Uso                                                  |
|-------------------------|------------------------|------------------------------------------------------|
| `logo.svg`              | **colorido** (verde+laranja) | logo **principal/padrão** (fundo neutro/claro) |
| `logo-branco.svg`       | branco                 | logo p/ fundo **escuro** (tema dark / faixa colorida)|
| `logo-preto.svg`        | preto                  | logo monocromático (impressão / contextos sem cor)   |
| `simbolo-branco.svg`    | branco                 | símbolo isolado p/ fundo **escuro**                  |
| `simbolo-preto.svg`     | preto                  | símbolo isolado p/ fundo **claro**                   |
| `og-image.svg`          | colorido               | compartilhamento (Open Graph) — hoje = `logo.svg`    |

> Paleta da marca: **laranja `#FF6600` (primária)**, verde `#33CC33`, roxo `#660066` (favicon).
> No Prompt 2.2 a variante de logo (colorida/branca/preta) é escolhida pelo tema automaticamente.
>
> ⚠️ `og-image.svg` está idêntico ao `logo.svg`. Para compartilhamento em redes o ideal é um
> **PNG ~1200×630 com fundo sólido** (a maioria dos scrapers ignora `og:image` em SVG). Baixa
> prioridade para uma ferramenta interna — deixar como está por enquanto é aceitável.

Referência num componente Leptos (Prompt 2.2):

```rust
view! { <img src="/images/logo.svg" alt="SuperCopo PCP" class="logo" /> }
```
