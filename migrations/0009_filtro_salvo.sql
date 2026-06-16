-- Filtros salvos por usuário na Gestão de Estoque (doc 03 §3.2). É PREFERÊNCIA de UI, não dado
-- de negócio: escopada ao dono (usuario_id) e removida junto com ele (CASCADE). O conteúdo do
-- filtro é opaco para o backend (jsonb) — quem o entende é o pcp-web (fronteira de módulo, §0).
-- Retenção (CLAUDE.md §9): enquanto o usuário existir / até exclusão pelo próprio.
CREATE TABLE pcp.filtro_salvo (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    usuario_id uuid NOT NULL REFERENCES pcp.usuario (id) ON DELETE CASCADE,
    nome       text NOT NULL,
    filtro     jsonb NOT NULL,
    criado_em  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT filtro_salvo_nome_nao_vazio CHECK (length(btrim(nome)) > 0),
    CONSTRAINT filtro_salvo_unico_por_usuario UNIQUE (usuario_id, nome)
);

CREATE INDEX filtro_salvo_usuario_idx ON pcp.filtro_salvo (usuario_id);
