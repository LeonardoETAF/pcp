//! Fonte de dados por arquivo CSV (doc 05 §2): lê e valida o contrato antes de produzir os
//! registros de entrada. O parsing é puro (testável sem banco).

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::Deserialize;

use pcp_db::{NovaVendaDia, NovoEstoqueSnapshot};

use crate::erro::ErroEtl;
use crate::fonte::FonteDados;

#[derive(Debug, Deserialize)]
struct LinhaVendaCsv {
    dt_ref: NaiveDate,
    codigo_estoque: String,
    #[serde(default)]
    sku: String,
    #[serde(default)]
    produto: String,
    #[serde(default)]
    configuracao: String,
    qtd_vendida: i32,
    is_personalizado: bool,
}

#[derive(Debug, Deserialize)]
struct LinhaSnapshotCsv {
    dt_ref: NaiveDate,
    codigo_estoque: String,
    #[serde(default)]
    sku: String,
    #[serde(default)]
    produto: String,
    #[serde(default)]
    configuracao: String,
    qtd_estoque: i32,
    qtd_reserva: i32,
    #[serde(default)]
    qtd_disponivel: Option<i32>,
    #[serde(default)]
    estoque_min_erp: Option<i32>,
    fora_de_linha: bool,
}

/// Importa os dados de entrada a partir de dois arquivos CSV (vendas e snapshot).
pub struct ImportadorArquivo {
    caminho_vendas: PathBuf,
    caminho_snapshot: PathBuf,
}

impl ImportadorArquivo {
    /// Cria o importador a partir dos caminhos dos CSV de vendas e de snapshot.
    #[must_use]
    pub fn novo(caminho_vendas: impl Into<PathBuf>, caminho_snapshot: impl Into<PathBuf>) -> Self {
        Self {
            caminho_vendas: caminho_vendas.into(),
            caminho_snapshot: caminho_snapshot.into(),
        }
    }
}

impl FonteDados for ImportadorArquivo {
    fn ler_vendas(&self) -> Result<Vec<NovaVendaDia>, ErroEtl> {
        ler_vendas_csv(abrir(&self.caminho_vendas)?)
    }

    fn ler_snapshots(&self) -> Result<Vec<NovoEstoqueSnapshot>, ErroEtl> {
        ler_snapshot_csv(abrir(&self.caminho_snapshot)?)
    }
}

fn abrir(caminho: &Path) -> Result<File, ErroEtl> {
    File::open(caminho).map_err(|origem| ErroEtl::Io {
        caminho: caminho.display().to_string(),
        origem,
    })
}

fn vazio_para_none(texto: String) -> Option<String> {
    if texto.trim().is_empty() {
        None
    } else {
        Some(texto)
    }
}

fn validar_codigo(codigo: &str, linha: usize) -> Result<(), ErroEtl> {
    if codigo.trim().is_empty() {
        Err(ErroEtl::Validacao {
            linha,
            motivo: "codigo_estoque vazio".to_owned(),
        })
    } else {
        Ok(())
    }
}

fn ler_vendas_csv<R: Read>(leitor: R) -> Result<Vec<NovaVendaDia>, ErroEtl> {
    let mut rdr = csv::Reader::from_reader(leitor);
    let mut vendas = Vec::new();
    for (indice, resultado) in rdr.deserialize::<LinhaVendaCsv>().enumerate() {
        let linha = indice + 2; // cabeçalho (1) + base 1
        let r = resultado?;
        validar_codigo(&r.codigo_estoque, linha)?;
        if r.qtd_vendida < 0 {
            return Err(ErroEtl::Validacao {
                linha,
                motivo: "qtd_vendida negativa (doc 05 §2.1)".to_owned(),
            });
        }
        vendas.push(NovaVendaDia {
            dt_ref: r.dt_ref,
            codigo_estoque: r.codigo_estoque,
            sku: vazio_para_none(r.sku),
            produto: vazio_para_none(r.produto),
            configuracao: vazio_para_none(r.configuracao),
            qtd_vendida: r.qtd_vendida,
            is_personalizado: r.is_personalizado,
        });
    }
    Ok(vendas)
}

fn ler_snapshot_csv<R: Read>(leitor: R) -> Result<Vec<NovoEstoqueSnapshot>, ErroEtl> {
    let mut rdr = csv::Reader::from_reader(leitor);
    let mut snapshots = Vec::new();
    for (indice, resultado) in rdr.deserialize::<LinhaSnapshotCsv>().enumerate() {
        let linha = indice + 2;
        let r = resultado?;
        validar_codigo(&r.codigo_estoque, linha)?;
        // Contrato doc 05 §2.2: disponivel = estoque − reserva (calcula se ausente).
        let esperado = r.qtd_estoque - r.qtd_reserva;
        let disponivel = r.qtd_disponivel.unwrap_or(esperado);
        if disponivel != esperado {
            return Err(ErroEtl::Validacao {
                linha,
                motivo: format!(
                    "qtd_disponivel ({disponivel}) != qtd_estoque - qtd_reserva ({esperado})"
                ),
            });
        }
        snapshots.push(NovoEstoqueSnapshot {
            dt_ref: r.dt_ref,
            codigo_estoque: r.codigo_estoque,
            sku: vazio_para_none(r.sku),
            produto: vazio_para_none(r.produto),
            configuracao: vazio_para_none(r.configuracao),
            qtd_estoque: r.qtd_estoque,
            qtd_reserva: r.qtd_reserva,
            qtd_disponivel: disponivel,
            estoque_min_erp: r.estoque_min_erp,
            fora_de_linha: r.fora_de_linha,
        });
    }
    Ok(snapshots)
}

#[cfg(test)]
mod testes {
    use super::{ler_snapshot_csv, ler_vendas_csv};

    const VENDAS: &str =
        "dt_ref,codigo_estoque,sku,produto,configuracao,qtd_vendida,is_personalizado\n\
        2026-06-14,6797,SKU1,Copo,COR DO PRODUTO: AZUL,10,false\n\
        2026-06-14,6797,SKU1,Copo,,3,true\n";

    #[test]
    fn parse_vendas_consolida_variacoes() {
        let v = ler_vendas_csv(VENDAS.as_bytes()).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].codigo_estoque, "6797");
        assert_eq!(v[0].qtd_vendida, 10);
        assert_eq!(v[1].configuracao, None); // vazio -> None
        assert!(v[1].is_personalizado);
    }

    #[test]
    fn vendas_qtd_negativa_falha() {
        let csv = "dt_ref,codigo_estoque,sku,produto,configuracao,qtd_vendida,is_personalizado\n\
            2026-06-14,6797,,,,-5,false\n";
        assert!(ler_vendas_csv(csv.as_bytes()).is_err());
    }

    #[test]
    fn vendas_codigo_vazio_falha() {
        let csv = "dt_ref,codigo_estoque,sku,produto,configuracao,qtd_vendida,is_personalizado\n\
            2026-06-14,,,,,10,false\n";
        assert!(ler_vendas_csv(csv.as_bytes()).is_err());
    }

    #[test]
    fn snapshot_calcula_disponivel_quando_ausente() {
        let csv = "dt_ref,codigo_estoque,sku,produto,configuracao,qtd_estoque,qtd_reserva,qtd_disponivel,estoque_min_erp,fora_de_linha\n\
            2026-06-15,6797,,,,100,30,,,false\n";
        let s = ler_snapshot_csv(csv.as_bytes()).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].qtd_disponivel, 70); // 100 - 30
        assert_eq!(s[0].estoque_min_erp, None);
    }

    #[test]
    fn snapshot_disponivel_inconsistente_falha() {
        // 50 != 100 - 30 -> viola o contrato (doc 05 §2.2).
        let csv = "dt_ref,codigo_estoque,sku,produto,configuracao,qtd_estoque,qtd_reserva,qtd_disponivel,estoque_min_erp,fora_de_linha\n\
            2026-06-15,6797,,,,100,30,50,,false\n";
        assert!(ler_snapshot_csv(csv.as_bytes()).is_err());
    }
}
