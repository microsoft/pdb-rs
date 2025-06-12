# Relationships between data: References 

Many of the data structures in PDBs are related to other each other. This can take many forms:

* Some streams contain stream indices of other streams.  If information is moved from one stream index to another (during a PDB rebuild), then the streams which pointed to the old stream numbers need to be updated to point to the new ones.
  + Example: The Debug Information (DBI) stream contains stream indices for Module Information streams, the Global Symbol Stream (GSS), the Global Symbol Index (GSI), etc.
  + Example: The PDB Information Stream contains stream indexes for named streams.
* Some records contain byte offsets that point into other streams.
  + Example: The GSI contains byte offsets that point into the contents of the GSS.
* Some records contain record indices that refer to records in other streams.
  + Example: Symbol records refer to type records using a TypeIndex.
* Some bounds (array sizes) or other parameters are stored in headers or separate streams.

For a PDB reader, these references are obviously needed to get the job done; it’s always obvious that you need to read the DBI before you can find the right stream that contains the GSI.

However, these references are much more important for apps that create or modify PDBs, because editing information may invalidate information. It is important to understand the relationships between all of the data in PDBs, so that the relationships can be preserved when creating or modifying PDBs. In some cases the edits desired are minor, such as adding a new NatVis XML file to an existing PDB. In other cases, like achieving determinism, we need a rigorous and formal approach to guarantee correctness. 

Formally, we define the reference graph of all data structures within a PDB. The reference graph is acyclic; it does not make sense for a data structure to depend on itself, directly or indirectly.

The reference graph allows us to reason about how to make changes to a PDB while preserving all the invariants of the PDB. The reference graph will answer questions, such as: If I change the order of the records in table T, what other tables do I need to update? And transitively, what other tables (indirect dependencies) do I need to update?

Achieving determinism requires a topology-walk through these dependencies in one direction.

This diagram illustrates many of the references that connect the modules, symbols, and types
data structures.

```mermaid
flowchart TB;

dbi[["DBI"]]
dbi_modules["DBI Module Info"]
dbi_sources["DBI Sources"]

subgraph "TPI and IPI"
    ipi[["IPI Stream"]]
    ipi_hash[["IPI Hash Stream"]]
    ipi_hash --"byte<br>offset"--> ipi

    tpi[["TPI Stream"]]
    tpi_hash[["TPI Hash Stream"]]
    tpi_hash --"byte<br>offset"--> tpi
end

gss[["GSS"]]
gsi[["GSI"]]
psi[["PSI"]]
names_stream[["Names Stream"]]

dbi_opt_headers["DBI Optional Headers"]
optional_dbg_header_stream[["Optional Debug Headers"]]

dbi --contains--> dbi_modules
dbi --contains--> dbi_sources
dbi --contains--> dbi_opt_headers

dbi_opt_headers --"StreamIndex"--> optional_dbg_header_stream

dbi --StreamIndex--> gss
dbi --StreamIndex--> gsi
dbi --StreamIndex--> psi

subgraph Globals
    gss
    gsi
    psi
end

subgraph module_stream_graph [Module Stream]
    module_stream[["Module Stream"]]
    module_stream_globalrefs["Module<br>GlobalRefs<br>Substream"]
    module_symbols["Module Symbols<br>Substream"]
    c13line["C13 Line Data<br>Substream"]

    module_stream --contains--> module_symbols
    module_stream --contains--> c13line
    module_stream --contains--> module_stream_globalrefs
end

dbi_modules --StreamIndex--> module_stream
dbi_sources --NameIndex--> names_stream

gsi --"byte offset"--> gss
psi --"byte offset"--> gss
gss --TypeIndex----> tpi

module_symbols --TypeIndex--> tpi
module_symbols --ItemId--> ipi

module_stream_globalrefs --"byte offset"--> gss

c13line --NameIndex--> names_stream
c13line --"Mod#,File"--> dbi_sources

ipi --TypeIndex--> tpi

pdbi[["PDB Info"]]
named["Named Streams"]
tmcache_map[["TMCache Map"]]
tmcache_stream[["TMCache Stream"]]
natvis_stream["NatVis Streams"]

pdbi --contains--> named
named --StreamIndex--> natvis_stream
named --StreamIndex--> tmcache_map
tmcache_map --"StreamIndex"--> tmcache_stream

named --StreamIndex--> srcsrv_stream["SrcSrv"]
named --StreamIndex--> names_stream
```

* Each node is a data structure.
* Edges represent pointers from one data structure to another, which need to be updated when
  the pointed-to data structure is modified.
* Double-boxed nodes represent streams.

References within nodes are _not_ shown. For example, the TPI Stream contains `TypeIndex` values,
which point into the TPI Stream. These self-edges are not shown in the diagram above, for the sake
of clarity.


