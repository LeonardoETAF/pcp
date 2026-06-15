# `public/icons/` — biblioteca de ícones de interface (UI)

Conjunto de **93 ícones de UI** em SVG, grade `0 0 24 24`, traço `currentColor` — herdam a cor
do tema (claro/escuro) automaticamente (§16). Cobrem todo o futuro ERP (produção, estoque,
financeiro, RH, qualidade, vendas, compras…); o PCP usa um subconjunto agora.

São servidos em `/icons/<nome>.svg`. Como serão consumidos com tema (decidido no Prompt 2.2):
- **CSS mask** (themable, cacheável): `mask-image: url(/icons/alerta.svg); background: currentColor;`
- **ou SVG inline** via `include_str!` em um componente Leptos, para herdar `currentColor`
  direto e zerar requisições nos ícones mais usados.

> `<img src="/icons/x.svg">` **não** herda `currentColor` — evitar onde o tema precisa pintar o ícone.

## Ícones do app/marca NÃO ficam aqui

Favicon e ícones de instalação (PWA/Apple) ficam na **raiz de `public/`** (ver `../README.md`),
separados desta biblioteca de UI.
