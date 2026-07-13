# Diagnóstico: Produtos Eco Label e Preços Progressivos

**Data:** 2026-01-22  
**Objetivo:** Analisar estrutura do schema `catalogo` e planejar cadastro de produtos eco label com preços progressivos por quantidade

---

## 📋 RESUMO EXECUTIVO

### ✅ O QUE JÁ EXISTE

1. **Schema `catalogo` completo e estruturado**
   - 14 tabelas principais
   - Sistema de categorias, cores, variantes
   - Estrutura de SKUs completos (item + cor + variante)
   - Sistema de preços com vigência

2. **Produtos similares já cadastrados**
   - Existem produtos "Eco" na categoria "Linha Eco"
   - Produtos encontrados: Eco 400 Liso, Eco 400 Degradê, Eco 600 Liso, Eco 600 Degradê, Eco Duo 500ml
   - **MAS:** Não são os produtos específicos "eco label" solicitados

3. **Estrutura de preços existente**
   - Tabela `precos` com preço unitário por SKU completo
   - Tabela `tabelas_preco` para diferentes estratégias (varejo, atacado, promocional)
   - Sistema de vigência de preços

### ❌ O QUE NÃO EXISTE

1. **Produtos eco label específicos**
   - ❌ Eco Label 400ml
   - ❌ Eco Label 500ml  
   - ❌ Eco Label 600ml
   - ❌ Eco Label 550ml + Tirante

2. **Estrutura de preços progressivos por quantidade**
   - Não há campo `quantidade_minima` na tabela `precos`
   - Não há campo `quantidade_maxima` na tabela `precos`
   - Não há tabela específica para faixas de quantidade
   - Não há sistema para consultar preço baseado em quantidade

---

## 🔍 ANÁLISE DETALHADA DO SCHEMA

### Tabelas Principais do Schema `catalogo`

| Tabela | Registros | Propósito |
|--------|-----------|-----------|
| `categorias` | 22 | Categorias de produtos (ex: Linha Eco, Canecas) |
| `itens` | 134 | Produtos e acessórios (tabela unificada) |
| `cores` | 604 | Cores disponíveis para produtos |
| `variantes_acabamento` | 33 | Variantes de acabamento (Fosco, Metalizado, etc) |
| `itens_cores_variantes` | 2.044 | SKUs completos (combinação item + cor + variante) |
| `tabelas_preco` | 1 | Tabelas de preço (varejo, atacado, promocional) |
| `precos` | 134 | Preços unitários por SKU completo |
| `embalagens` | 161 | Embalagens disponíveis |
| `itens_embalagens` | 308 | Relacionamento itens x embalagens |

### Estrutura da Tabela `itens`

```sql
CREATE TABLE catalogo.itens (
    id UUID PRIMARY KEY,
    categoria_id UUID NOT NULL,
    slug TEXT UNIQUE NOT NULL,           -- Identificador URL-friendly
    nome TEXT NOT NULL,
    descricao TEXT,
    tipo_item TEXT NOT NULL,             -- 'produto' ou 'acessorio'
    linha_produto TEXT,                  -- Premium, Light, Fit, Bio, Fosco, Metalizado
    sku_base TEXT,                       -- SKU base no ERP
    erp_id TEXT UNIQUE,
    capacidade_ml INTEGER,               -- Capacidade em ml
    unidade_medida TEXT DEFAULT 'UN',
    peso_unitario_g DECIMAL(10,2),
    dimensoes_cm JSONB,
    metadata JSONB DEFAULT '{}',         -- Metadados adicionais
    ativo BOOLEAN DEFAULT true,
    visivel_catalogo BOOLEAN DEFAULT true,
    ...
);
```

### Estrutura da Tabela `precos`

```sql
CREATE TABLE catalogo.precos (
    id UUID PRIMARY KEY,
    item_cor_variante_id UUID NOT NULL,  -- FK para SKU completo
    tabela_preco_id UUID NOT NULL,      -- FK para tabela de preço
    preco_unitario DECIMAL(10,2) NOT NULL,
    custo_referencia DECIMAL(10,2),
    desconto_percentual DECIMAL(5,2) DEFAULT 0,
    vigencia_inicio DATE NOT NULL,
    vigencia_fim DATE,
    ativo BOOLEAN DEFAULT true,
    ...
    -- ❌ FALTANDO: quantidade_minima, quantidade_maxima
);
```

### Estrutura da Tabela `tabelas_preco`

```sql
CREATE TABLE catalogo.tabelas_preco (
    id UUID PRIMARY KEY,
    nome TEXT NOT NULL,
    descricao TEXT,
    tipo_tabela TEXT NOT NULL,          -- 'varejo', 'atacado', 'promocional', 'especial'
    vigencia_inicio DATE NOT NULL,
    vigencia_fim DATE,
    ativo BOOLEAN DEFAULT true,
    ...
);
```

**Tabela existente:** "Atacado Liso" (tipo: atacado, vigência: 2025-10-01 até 2026-12-31)

---

## 📦 PRODUTOS ECO LABEL - ANÁLISE

### Produtos Esperados vs Encontrados

| Produto | Status | Observações |
|---------|--------|-------------|
| Eco Label 400ml | ❌ Não encontrado | Existe "Eco 400 Liso" mas não é "eco label" |
| Eco Label 500ml | ❌ Não encontrado | Existe "Eco Duo 500ml" mas não é "eco label" |
| Eco Label 600ml | ❌ Não encontrado | Existe "Eco 600 Liso" mas não é "eco label" |
| Eco Label 550ml + Tirante | ❌ Não encontrado | Não existe produto 550ml com tirante |

### Produtos Similares Encontrados

**Linha Eco (categoria existente):**
- `eco-400-liso` - Eco 400 Liso 400ml
- `eco-400-degrade-liso` - Eco 400 Degradê 400ml
- `eco-duo-liso` - Eco Duo Liso 500ml
- `eco-600-liso` - Eco 600 Liso 600ml
- `eco-600-degrade-liso` - Eco 600 Degradê 600ml

**Características dos produtos Eco existentes:**
- Categoria: "Linha Eco"
- Metadata padrão: `linha_eco: true`
- Capacidades: 400ml, 500ml, 600ml
- Acabamentos: Liso, Degradê
- Espessura: 1.3mm (400ml e 600ml), 2.0mm (500ml - parede dupla)

---

## 💰 ANÁLISE DE PREÇOS PROGRESSIVOS

### Requisitos Baseados no Anexo 1

**Estrutura de Preços Progressivos (exemplo Eco 400 Label):**

| Quantidade | Preço Unitário |
|------------|----------------|
| 30 unidades | R$ 3.95 |
| 200 unidades | R$ 3.80 |
| 500 unidades | R$ 3.65 |
| 800 unidades | R$ 3.55 |
| 1.000 unidades | R$ 3.50 |
| 3.000 unidades | R$ 3.30 |
| 5.000 unidades | R$ 3.15 |
| 7.000 unidades | R$ 3.05 |
| 10.000 unidades | R$ 2.95 |
| 15.000 unidades | R$ 2.85 |
| 20.000 unidades | R$ 2.75 |
| 50.000 unidades | R$ 2.60 |

**Regras:**
- Pedido mínimo: 30 unidades
- Múltiplos de 10 unidades
- Preço progressivo: quanto mais unidades, menor o preço unitário

### Estrutura Atual vs Necessária

**Estrutura Atual (`precos`):**
- ✅ Preço unitário por SKU completo
- ✅ Vigência de preços
- ✅ Múltiplas tabelas de preço
- ❌ **FALTA:** Faixas de quantidade
- ❌ **FALTA:** Quantidade mínima/máxima

**Estrutura Necessária:**
- Preço unitário variável por faixa de quantidade
- Quantidade mínima e máxima por faixa
- Consulta eficiente: dado SKU + quantidade → retornar preço correto

---

## 🎯 OPÇÕES DE IMPLEMENTAÇÃO

### OPÇÃO 1: Adicionar Campos na Tabela `precos` ⭐ **RECOMENDADA**

**Vantagens:**
- Mantém compatibilidade com estrutura existente
- Consultas eficientes (índices diretos)
- Facilita criação de RPCs
- Não requer nova tabela

**Implementação:**
```sql
ALTER TABLE catalogo.precos
ADD COLUMN quantidade_minima INTEGER,
ADD COLUMN quantidade_maxima INTEGER;

-- Criar múltiplos registros por SKU com diferentes faixas
-- Exemplo para Eco Label 400ml:
-- Registro 1: quantidade_minima=30, quantidade_maxima=199, preco_unitario=3.95
-- Registro 2: quantidade_minima=200, quantidade_maxima=499, preco_unitario=3.80
-- etc.
```

**RPC Sugerida:**
```sql
CREATE FUNCTION catalogo.obter_preco_por_quantidade(
    p_item_cor_variante_id UUID,
    p_quantidade INTEGER,
    p_tabela_preco_id UUID DEFAULT NULL
) RETURNS DECIMAL(10,2)
```

### OPÇÃO 2: Criar Nova Tabela `precos_faixas_quantidade`

**Vantagens:**
- Estrutura clara e dedicada
- Separação de responsabilidades
- Facilita evoluções futuras

**Desvantagens:**
- Requer nova tabela e migração
- Mais complexo para consultas

**Implementação:**
```sql
CREATE TABLE catalogo.precos_faixas_quantidade (
    id UUID PRIMARY KEY,
    item_cor_variante_id UUID NOT NULL,
    tabela_preco_id UUID NOT NULL,
    quantidade_minima INTEGER NOT NULL,
    quantidade_maxima INTEGER,
    preco_unitario DECIMAL(10,2) NOT NULL,
    vigencia_inicio DATE NOT NULL,
    vigencia_fim DATE,
    ativo BOOLEAN DEFAULT true,
    ...
);
```

### OPÇÃO 3: Usar Metadata JSONB

**Vantagens:**
- Não requer mudança de schema
- Flexível para diferentes estruturas

**Desvantagens:**
- Menos performático para consultas
- Dificulta validação e integridade
- Não recomendado para dados críticos

---

## 📝 PLANO DE IMPLEMENTAÇÃO RECOMENDADO

### FASE 1: Preparação do Schema

1. **Adicionar campos na tabela `precos`**
   ```sql
   ALTER TABLE catalogo.precos
   ADD COLUMN quantidade_minima INTEGER,
   ADD COLUMN quantidade_maxima INTEGER;
   
   CREATE INDEX idx_precos_quantidade 
   ON catalogo.precos(item_cor_variante_id, quantidade_minima, quantidade_maxima)
   WHERE ativo = true;
   ```

2. **Criar RPC para consulta de preço por quantidade**
   ```sql
   CREATE FUNCTION catalogo.obter_preco_por_quantidade(
       p_item_cor_variante_id UUID,
       p_quantidade INTEGER,
       p_tabela_preco_id UUID DEFAULT NULL
   ) RETURNS TABLE (
       preco_unitario DECIMAL(10,2),
       quantidade_minima INTEGER,
       quantidade_maxima INTEGER
   )
   ```

### FASE 2: Cadastro de Produtos

1. **Verificar/Criar categoria "Eco Label"**
   - Se não existir, criar categoria específica
   - Ou usar categoria existente "Linha Eco"

2. **Cadastrar produtos eco label**
   - Eco Label 400ml (slug: `eco-label-400ml`)
   - Eco Label 500ml (slug: `eco-label-500ml`)
   - Eco Label 600ml (slug: `eco-label-600ml`)
   - Eco Label 550ml + Tirante (slug: `eco-label-550ml-tirante`)

3. **Criar SKUs completos**
   - Para cada produto, criar combinações item + cor + variante
   - Considerar cores padrão (se aplicável)

### FASE 3: Cadastro de Preços Progressivos

1. **Criar tabela de preço específica** (opcional)
   - Nome: "Eco Label - Progressivo"
   - Tipo: "atacado" ou "especial"

2. **Cadastrar faixas de preço**
   - Para cada SKU completo, criar múltiplos registros na tabela `precos`
   - Cada registro representa uma faixa de quantidade
   - Exemplo para Eco Label 400ml:
     - 30-199 unidades: R$ 3.95
     - 200-499 unidades: R$ 3.80
     - 500-799 unidades: R$ 3.65
     - etc.

### FASE 4: Integração com Fluxo Gamificado

1. **Criar serviço no frontend**
   - Função para buscar preço por produto + quantidade
   - Chamar RPC `catalogo.obter_preco_por_quantidade`

2. **Atualizar fluxo gamificado**
   - Quando lead escolhe produto e quantidade
   - Buscar preço correspondente
   - Exibir no resumo do pedido

---

## 🔧 DETALHES TÉCNICOS

### Estrutura de Dados para Produtos Eco Label

**Exemplo: Eco Label 400ml**

```json
{
  "nome": "Eco Label 400ml",
  "slug": "eco-label-400ml",
  "categoria": "Linha Eco",
  "capacidade_ml": 400,
  "tipo_item": "produto",
  "metadata": {
    "linha_eco": true,
    "linha_eco_label": true,
    "tipo": "eco_label",
    "espessura_mm": 1.3,
    "pedido_minimo": 30,
    "multiplos_de": 10,
    "permite_borda_metalizada": false
  }
}
```

### Estrutura de Preços Progressivos

**Exemplo: Preços para Eco Label 400ml (SKU específico)**

| ID | SKU | Qtd Mín | Qtd Máx | Preço Unitário | Tabela |
|----|-----|---------|---------|----------------|--------|
| 1 | ECO-LABEL-400-LISO-AZUL | 30 | 199 | 3.95 | Eco Label Progressivo |
| 2 | ECO-LABEL-400-LISO-AZUL | 200 | 499 | 3.80 | Eco Label Progressivo |
| 3 | ECO-LABEL-400-LISO-AZUL | 500 | 799 | 3.65 | Eco Label Progressivo |
| ... | ... | ... | ... | ... | ... |

### Query de Consulta de Preço

```sql
SELECT 
    preco_unitario,
    quantidade_minima,
    quantidade_maxima
FROM catalogo.precos
WHERE item_cor_variante_id = $1
  AND quantidade_minima <= $2
  AND (quantidade_maxima IS NULL OR quantidade_maxima >= $2)
  AND ativo = true
  AND vigencia_inicio <= CURRENT_DATE
  AND (vigencia_fim IS NULL OR vigencia_fim >= CURRENT_DATE)
ORDER BY quantidade_minima DESC
LIMIT 1;
```

---

## ✅ CHECKLIST DE IMPLEMENTAÇÃO

### Schema e Estrutura
- [ ] Adicionar campos `quantidade_minima` e `quantidade_maxima` na tabela `precos`
- [ ] Criar índice para consultas eficientes
- [ ] Criar RPC `catalogo.obter_preco_por_quantidade`
- [ ] Criar wrapper público `public.catalogo_obter_preco_por_quantidade`

### Produtos
- [ ] Verificar/Criar categoria "Eco Label" ou usar "Linha Eco"
- [ ] Cadastrar produto "Eco Label 400ml"
- [ ] Cadastrar produto "Eco Label 500ml"
- [ ] Cadastrar produto "Eco Label 600ml"
- [ ] Cadastrar produto "Eco Label 550ml + Tirante"
- [ ] Criar SKUs completos para cada produto (cores padrão)

### Preços
- [ ] Criar tabela de preço "Eco Label - Progressivo"
- [ ] Cadastrar faixas de preço para Eco Label 400ml
- [ ] Cadastrar faixas de preço para Eco Label 500ml
- [ ] Cadastrar faixas de preço para Eco Label 600ml
- [ ] Cadastrar faixas de preço para Eco Label 550ml + Tirante

### Integração
- [ ] Criar serviço TypeScript para consulta de preços
- [ ] Integrar com fluxo gamificado
- [ ] Testar consulta de preço por quantidade
- [ ] Validar cálculo de preço total

---

## 📊 MÉTRICAS E VALIDAÇÕES

### Validações Necessárias

1. **Validação de Quantidade**
   - Verificar se quantidade é múltiplo de 10
   - Verificar se quantidade >= 30 (pedido mínimo)

2. **Validação de Preço**
   - Garantir que sempre existe preço para quantidade informada
   - Validar que preço progressivo está correto (menor quantidade = maior preço)

3. **Validação de SKU**
   - Verificar se SKU completo existe e está ativo
   - Verificar se produto está visível no catálogo

---

## 🚀 PRÓXIMOS PASSOS

1. **Aprovação do plano de implementação**
2. **Criação de migration para adicionar campos na tabela `precos`**
3. **Criação de RPC para consulta de preços**
4. **Cadastro de produtos eco label**
5. **Cadastro de preços progressivos**
6. **Testes e validação**
7. **Integração com fluxo gamificado**

---

**Documento criado em:** 2026-01-22  
**Última atualização:** 2026-01-22
