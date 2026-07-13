# Implementação: Campos de Quantidade e Preparação para Cadastro

**Data:** 2026-01-22  
**Status:** ✅ Migration criada, scripts de cadastro prontos

---

## ✅ O QUE FOI IMPLEMENTADO

### 1. Migration: Campos de Quantidade

**Arquivo:** `supabase/migrations/20260122000004_adicionar_campos_quantidade_precos.sql`

**O que faz:**
- Adiciona campos `quantidade_minima` e `quantidade_maxima` na tabela `catalogo.precos`
- Cria índices para consultas eficientes
- Adiciona constraint para validar faixas de quantidade
- Adiciona comentários nas colunas

**Campos adicionados:**
```sql
quantidade_minima INTEGER  -- Quantidade mínima para esta faixa de preço
quantidade_maxima INTEGER  -- Quantidade máxima para esta faixa de preço
```

**Índices criados:**
- `idx_precos_quantidade` - Para consultas gerais
- `idx_precos_quantidade_busca` - Para busca rápida por quantidade específica

**Constraint:**
- `chk_quantidade_faixa_valida` - Valida que quantidade_maxima >= quantidade_minima

### 2. Scripts de Preparação e Cadastro

#### `scripts/catalogo/preparar_cadastro_eco_label.py`
- Verifica se campos de quantidade existem
- Verifica categoria "Linha Eco"
- Prepara dados dos 4 produtos eco label
- Verifica se produtos já existem

#### `scripts/catalogo/cadastrar_produto_eco_label.py`
- Script interativo para cadastrar produtos eco label
- Permite cadastrar um produto por vez ou todos de uma vez
- Valida se produto já existe antes de cadastrar
- Usa categoria "Linha Eco" automaticamente

#### `scripts/catalogo/aplicar_migration_campos_quantidade.py`
- Script para aplicar a migration no banco de produção

---

## 📋 PRODUTOS PREPARADOS PARA CADASTRO

### 1. Eco Label 400ml
- **Slug:** `eco-label-400ml`
- **SKU Base:** `ECOL400`
- **Capacidade:** 400ml
- **Metadata:** Configurado com pedido mínimo 30, múltiplos de 10

### 2. Eco Label 500ml
- **Slug:** `eco-label-500ml`
- **SKU Base:** `ECOL500`
- **Capacidade:** 500ml
- **Metadata:** Configurado com pedido mínimo 30, múltiplos de 10

### 3. Eco Label 600ml
- **Slug:** `eco-label-600ml`
- **SKU Base:** `ECOL600`
- **Capacidade:** 600ml
- **Metadata:** Configurado com pedido mínimo 30, múltiplos de 10

### 4. Eco Label 550ml + Tirante
- **Slug:** `eco-label-550ml-tirante`
- **SKU Base:** `ECOL550T`
- **Capacidade:** 550ml
- **Metadata:** Configurado com pedido mínimo 30, múltiplos de 10, com_tirante: true

---

## 🚀 COMO USAR

### Passo 1: Aplicar Migration

```bash
# Opção 1: Usar script Python
python scripts/catalogo/aplicar_migration_campos_quantidade.py

# Opção 2: Aplicar manualmente via Supabase CLI ou SQL direto
# Arquivo: supabase/migrations/20260122000004_adicionar_campos_quantidade_precos.sql
```

### Passo 2: Verificar Preparação

```bash
python scripts/catalogo/preparar_cadastro_eco_label.py
```

Este script vai mostrar:
- ✅ Se campos de quantidade existem
- ✅ Categoria encontrada (Linha Eco)
- ✅ Lista de produtos preparados
- ✅ Se produtos já existem

### Passo 3: Cadastrar Produtos

```bash
python scripts/catalogo/cadastrar_produto_eco_label.py
```

O script é interativo e permite:
- Cadastrar um produto por vez (opções 1-4)
- Cadastrar todos de uma vez (opção 0)
- Validação automática de duplicatas

---

## 📊 ESTRUTURA DE DADOS

### Tabela `catalogo.precos` (após migration)

```sql
CREATE TABLE catalogo.precos (
    id UUID PRIMARY KEY,
    item_cor_variante_id UUID NOT NULL,
    tabela_preco_id UUID NOT NULL,
    preco_unitario DECIMAL(10,2) NOT NULL,
    quantidade_minima INTEGER,        -- ✅ NOVO
    quantidade_maxima INTEGER,        -- ✅ NOVO
    custo_referencia DECIMAL(10,2),
    desconto_percentual DECIMAL(5,2),
    vigencia_inicio DATE NOT NULL,
    vigencia_fim DATE,
    ativo BOOLEAN DEFAULT true,
    ...
);
```

### Exemplo de Uso: Preços Progressivos

Para cadastrar preços progressivos, você criará múltiplos registros:

```sql
-- Faixa 1: 30-199 unidades
INSERT INTO catalogo.precos (
    item_cor_variante_id,
    tabela_preco_id,
    preco_unitario,
    quantidade_minima,
    quantidade_maxima,
    vigencia_inicio,
    ativo
) VALUES (
    'sku-id-aqui',
    'tabela-preco-id',
    3.95,
    30,
    199,
    CURRENT_DATE,
    true
);

-- Faixa 2: 200-499 unidades
INSERT INTO catalogo.precos (...) VALUES (..., 3.80, 200, 499, ...);

-- Faixa 3: 500-799 unidades
INSERT INTO catalogo.precos (...) VALUES (..., 3.65, 500, 799, ...);

-- E assim por diante...
```

---

## ✅ CHECKLIST DE IMPLEMENTAÇÃO

### Schema
- [x] Migration criada para adicionar campos
- [x] Índices criados para performance
- [x] Constraints de validação adicionadas
- [ ] Migration aplicada no banco (aguardando execução)

### Scripts
- [x] Script de preparação criado
- [x] Script de cadastro criado
- [x] Script de aplicação de migration criado

### Produtos
- [x] Dados dos 4 produtos preparados
- [x] Categoria identificada (Linha Eco)
- [ ] Produtos cadastrados (aguardando execução)

### Próximos Passos
- [ ] Aplicar migration
- [ ] Cadastrar produtos eco label
- [ ] Criar SKUs completos (item + cor + variante)
- [ ] Criar tabela de preço "Eco Label - Progressivo"
- [ ] Cadastrar preços progressivos por quantidade
- [ ] Criar RPC para consulta de preço por quantidade (deixado para depois)

---

## 📝 NOTAS

1. **RPC de Consulta:** Deixada para depois conforme solicitado
2. **Categoria:** Usa categoria existente "Linha Eco" (ID: `945ebc87-9c2d-47b6-aa1d-05b61c4f1b4e`)
3. **Validação:** Scripts validam duplicatas antes de cadastrar
4. **Metadata:** Todos os produtos têm metadata completa com regras de negócio

---

**Última atualização:** 2026-01-22
