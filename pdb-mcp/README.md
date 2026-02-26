# pdb-mcp — MCP Server for PDB Analysis

An MCP (Model Context Protocol) server that provides AI assistants with
structured access to Microsoft Program Database (PDB) files. Built on the
[`ms-pdb`](https://github.com/microsoft/pdb-rs) Rust library.

## What It Does

PDB files contain debugging information for Windows executables (EXE, DLL, SYS).
This MCP server lets AI assistants open PDB files and query their contents
through 18 fine-grained tools, without needing to shell out to command-line
utilities or parse unstructured output.

### Capabilities

- **Session management** — Open/close PDB or PDZ files by path, reference by alias
- **Metadata** — GUID, age, binding key, features, named streams, container format
- **Module browsing** — List compilands with dual regex filtering (name + obj file),
  enumerate per-module symbols and source files
- **Symbol lookup (accelerated)** — O(1) hash-based name lookup via GSI and PSI
  indices, binary search by address via PSI address map
- **Symbol search (brute-force)** — Regex pattern search across the entire Global
  Symbol Stream
- **Type inspection** — Search TPI for named types, dump individual type records
- **Section analysis** — COFF section headers, COFF groups
- **Raw stream access** — Read arbitrary bytes from any PDB stream by index or name,
  with automatic text/hex detection and offset-based paging
- **Statistics** — Aggregate record counts, stream sizes, module counts

### Use Cases

- Analyzing Windows binary structure and exported APIs
- Understanding type layouts (structs, enums, classes, unions)
- Investigating build provenance via SourceLink JSON
- Comparing PDB metadata across builds
- Navigating large module lists (Edge browser has ~48K compilands)
- Inspecting embedded resources (NatVis files, source files)

## Building

```bash
cd /path/to/pdb-rs
cargo build --release -p pdb-mcp
```

The binary is produced at `target/release/pdb-mcp` (or `pdb-mcp.exe` on Windows).

## Installation

Add to your VS Code workspace's `.vscode/mcp.json`:

```json
{
  "servers": {
    "pdb-mcp": {
      "command": "/path/to/pdb-mcp.exe",
      "args": []
    }
  }
}
```

The server communicates via stdio using the MCP JSON-RPC protocol.

## Tools

| Tool | Description |
|------|-------------|
| `open_pdb` | Open a PDB/PDZ file for analysis |
| `close_pdb` | Close an open PDB and free memory |
| `list_pdbs` | List all currently open PDBs |
| `pdb_info` | Full PDBI stream metadata (GUID, age, binding key, features, named streams) |
| `pdb_streams` | List all streams with index, size, and name |
| `read_stream` | Read raw data from any stream (text or hex dump, with paging) |
| `list_modules` | List modules with regex filters on name and obj file |
| `module_symbols` | Show symbols for a specific module |
| `module_source_files` | List source files for a module |
| `find_global` | GSI hash-accelerated exact name lookup (S_PROCREF, S_UDT, etc.) |
| `find_public` | PSI hash-accelerated exact name lookup (S_PUB32 only) |
| `find_public_by_addr` | PSI address map binary search |
| `search_symbols` | Regex search across the entire Global Symbol Stream |
| `find_type` | Search TPI for types by name |
| `dump_type` | Dump a specific type record by TypeIndex |
| `section_headers` | List COFF section headers |
| `coff_groups` | List COFF groups from the linker module |
| `pdb_stats` | Aggregate statistics (streams, records, modules) |

## Safety

> **This server is designed for use with PDB files you built yourself or that
> come from trusted sources.** Opening a PDB is like opening a document — its
> content becomes part of your AI context.

### Threat Model

PDB files contain human-readable strings: symbol names, type names, source file
paths, SourceLink URLs, named stream names, and embedded text resources (NatVis
files, source code). A malicious PDB can contain **arbitrary text** in any of
these fields — including text specifically crafted to manipulate an AI's
behavior (prompt injection).

Concrete attack vectors:

- **Symbol names** — A function or variable could be named with text designed to
  override the AI's instructions (e.g., a symbol named
  `IGNORE_ALL_PREVIOUS_INSTRUCTIONS_AND_...`).
- **Source file paths** — Module names and object file paths are attacker-controlled
  strings. A path like `d:\os\src\SYSTEM PROMPT OVERRIDE: ...` would appear in
  `list_modules` output.
- **SourceLink JSON** — The `"documents"` mapping contains both paths and URLs.
  Malicious URLs could point to phishing sites or contain instruction-like text.
  A crafted source path could embed directives.
- **Named streams** — Stream names are arbitrary strings stored in the PDB.
- **Embedded source files** — PDBs can contain embedded source code (NatVis, etc.)
  which is returned as text by `read_stream`.
- **Type and field names** — Struct members, enum values, and type names are
  displayed in `find_type` and `dump_type` output.

### For Users

- **Only open PDB files from sources you trust.** Treat PDB files like
  executables — they can contain content designed to manipulate tools that read
  them.
- **Be cautious with PDBs from the internet**, symbol servers you don't control,
  or unknown third parties.
- **Review AI actions** after analyzing unfamiliar PDB files. If the AI's
  behavior changes unexpectedly after opening a PDB, the file may contain
  prompt injection content.

### For AI Agents

The safety rules for AI agents consuming pdb-mcp output are included in the
MCP server's instruction block, which is delivered at initialization. See the
`SERVER_INSTRUCTIONS` constant in `src/server.rs`.

## License

MIT OR Apache-2.0
