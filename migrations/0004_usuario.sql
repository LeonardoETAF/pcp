-- Usuários e autenticação (CLAUDE.md §7). Auth é núcleo comum (§0); por ora no schema pcp.
CREATE TABLE pcp.usuario (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email         text NOT NULL UNIQUE,
    senha_hash    text NOT NULL,
    papel         text NOT NULL,
    nome          text,
    ativo         boolean NOT NULL DEFAULT true,
    criado_em     timestamptz NOT NULL DEFAULT now(),
    atualizado_em timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT usuario_papel_valido CHECK (papel IN ('analista', 'gestor', 'admin')),
    CONSTRAINT usuario_email_nao_vazio CHECK (length(btrim(email)) > 0)
);

-- Refresh tokens REVOGÁVEIS (CLAUDE.md §7.7). Guardamos só o HASH do token, nunca o valor.
CREATE TABLE pcp.refresh_token (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    usuario_id uuid NOT NULL REFERENCES pcp.usuario (id) ON DELETE CASCADE,
    token_hash text NOT NULL UNIQUE,
    expira_em  timestamptz NOT NULL,
    revogado   boolean NOT NULL DEFAULT false,
    criado_em  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX refresh_token_usuario_idx ON pcp.refresh_token (usuario_id);
