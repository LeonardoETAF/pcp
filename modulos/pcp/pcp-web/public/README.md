# `public/` — assets estáticos do frontend

Este diretório é o **`assets-dir` do `cargo-leptos`**: tudo aqui é copiado **verbatim para a
raiz do site** no build. O caminho relativo vira a URL pública.

| Arquivo no repositório            | URL servida                  |
|-----------------------------------|------------------------------|
| `public/favicon.png`              | `/favicon.png`               |
| `public/favicon.svg` (a criar)    | `/favicon.svg`               |
| `public/icon-192.png` (a criar)   | `/icon-192.png` (PWA)        |
| `public/manifest.webmanifest`     | `/manifest.webmanifest`      |
| `public/icons/alerta.svg`         | `/icons/alerta.svg`          |
| `public/images/logo.png`          | `/images/logo.png`           |

Organização:
- **Raiz de `public/`** — identidade do **app**: `favicon.*` e ícones de instalação
  (`icon-192.png`, `icon-512.png`, `icon-512-maskable.png`, `apple-touch-icon.png`),
  referenciados pelo `<head>` e pelo `manifest.webmanifest`.
- **`icons/`** — **biblioteca de ícones de UI** (93 SVGs `currentColor`). Ver `icons/README.md`.
- **`images/`** — **logo** e imagens de conteúdo (símbolos, og-image). Ver `images/README.md`.

A ligação `assets-dir = "public"` + `style-file` será adicionada ao `[package.metadata.leptos]`
quando o frontend ganhar as dependências do Leptos (Prompt 2.2).
