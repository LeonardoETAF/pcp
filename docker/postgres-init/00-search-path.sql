-- Roda uma vez, na criação do banco (/docker-entrypoint-initdb.d).
-- Fixa um search_path determinístico para o role do PCP: como o role e o schema se chamam
-- ambos `pcp`, o `$user` faria o `_sqlx_migrations` (não qualificado) cair no schema `pcp`.
-- Com `public` à frente, o controle de migrations vive sempre em `public`; as tabelas do
-- módulo são sempre referenciadas como `pcp.*` (CLAUDE.md §0).
ALTER ROLE pcp SET search_path TO public, pcp;
