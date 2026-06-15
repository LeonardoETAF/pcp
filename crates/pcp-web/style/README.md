# `style/` — CSS à mão (design tokens)

Estilo **100% CSS escrito à mão** (sem Node/Tailwind — CLAUDE.md §1/§16). O `cargo-leptos`
empacota o `style-file` daqui. A fundação (tokens de cor/tipografia/espaçamento, tema
claro/escuro, componentes) entra no **Prompt 2.2**; por ora a pasta só reserva o lugar.

Arquivo de entrada previsto: `style/main.css` (importará os tokens). A cor de marca definida
nos tokens deve casar com `theme_color`/`background_color` do `public/manifest.webmanifest`.
