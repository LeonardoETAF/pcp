//! Componentes reutilizáveis do design system (CLAUDE.md §16). Um componente por arquivo.

pub mod estado_vazio;
pub mod icone;
pub mod paginacao;
pub mod seletor;

pub use estado_vazio::EstadoVazio;
pub use icone::Icone;
pub use paginacao::PaginacaoBotoes;
pub use seletor::Seletor;
