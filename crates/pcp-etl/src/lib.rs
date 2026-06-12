//! ETL: ingestão pela fonte de dados atrás do trait `FonteDados` (hoje arquivo/CSV;
//! conector ao ERP "One" depois). Valida o contrato do doc 05 §2. (CLAUDE.md §1/§8)
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

// Esqueleto: o ImportadorArquivo e o trait FonteDados entram no prompt 1.7.
