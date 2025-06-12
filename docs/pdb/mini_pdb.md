# Mini PDBs

The MSVC linker provides a feature known as "fast PDBs" or "mini PDBs". It is
enabled using the `/DEBUG:FASTLINK` linker option. This option generates PDBs
that contain only minimal debug information, some of which consists of pointers
to debug information in other files.

Mini PDBs were intended to reduce developer inner loop times by shortening
linking times. However, recent improvements in linker performance have reduced
the performance gap between full PDBs and mini PDBs. `/DEBUG:FASTLINK` is now
considered a deprecated feature.

A mini PDB file can be identified by the presence of the `MinimalDebugInfo`
feature code in the PDB Information Stream.

The `MinimalDebugInfo` feature code determines the value of the `num_buckets`
parameter in the GSI and PSI Name Tables. It is therefore required in order to
decode those tables. If the `MinimalDebugInfo` feature is _absent_, then the
value of `num_buckets` is 0x1000. If the `MinimalDebugInfo` feature is
_present_, then the value of `num_buckets` is 0x3ffff.

# References

* [/DEBUG (Generate debug info)](https://learn.microsoft.com/en-us/cpp/build/reference/debug-generate-debug-info)
