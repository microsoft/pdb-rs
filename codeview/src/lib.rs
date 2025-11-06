//! CodeView definitions
//!
//! This defines types and constants for CodeView debugging tools. CodeView is the debugging
//! format used for PE/COFF images on Windows.
//!
//! This crate does not provide any I/O capabilities. It does not read or write PDBs, `.debug`
//! sections in PE/COFF objects, etc.
//!
//! * [`ms-pdb`](https://crates.io/crates/ms-pdb) - Use this crate for reading and writing PDB files.
//!
//! # References
//!
//! * [CodeView Symbols](https://llvm.org/docs/PDB/CodeViewSymbols.html)
//! * [`cvinfo.h`](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/include/cvinfo.h)

#![forbid(unsafe_code)]
#![forbid(unused_must_use)]
#![warn(missing_docs)]
#![allow(clippy::needless_lifetimes)]

pub mod arch;
pub mod encoder;
pub mod parser;
pub mod syms;
pub mod types;
mod utils;

pub use utils::iter::*;
