-- Núcleo do SuperFlow — IDENTIDADE (schema `nucleo`).
--
-- Autenticação e usuários são do NÚCLEO, não do PCP: o CLAUDE.md §0 diz que cada módulo
-- se pluga a "um núcleo comum (auth, config, usuários, infra de dados)". Enquanto essas
-- tabelas viviam em `pcp.*`, o segundo módulo (Catálogo) só poderia autenticar acoplando-se
-- ao PCP — proibido pelo §0/§13. Aqui elas passam para o schema do núcleo.
--
-- `ALTER TABLE ... SET SCHEMA` MOVE a tabela preservando dados, índices, constraints e as
-- 8 chaves estrangeiras que apontam para `usuario` (o Postgres reaponta as dependências
-- sozinho). Não há recriação nem cópia — portanto nenhuma perda de usuário ou de sessão.
--
-- ORDEM DE EXECUÇÃO: esta migration roda DEPOIS das do módulo PCP. Quem ainda CRIA a tabela
-- é a `0004_usuario.sql` do PCP — migration já aplicada em produção, cujo conteúdo não pode
-- ser reescrito sem quebrar o checksum do sqlx. Aqui ela só troca de dono. Quando as
-- migrations do PCP forem consolidadas (squash), o DDL nasce direto aqui e esta dependência
-- de ordem some. Enquanto isso: uma instalação SEM o módulo PCP ainda não é possível — é o
-- que fecharemos ao criar o app do Catálogo.

CREATE SCHEMA IF NOT EXISTS nucleo;

-- Idempotente: só move se ainda estiver no schema antigo (reprocesso/reinstalação limpa).
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'pcp' AND table_name = 'usuario'
    ) THEN
        ALTER TABLE pcp.usuario SET SCHEMA nucleo;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'pcp' AND table_name = 'refresh_token'
    ) THEN
        ALTER TABLE pcp.refresh_token SET SCHEMA nucleo;
    END IF;
END $$;

COMMENT ON SCHEMA nucleo IS
    'Núcleo compartilhado do SuperFlow: identidade e infra comum a todos os módulos.';
