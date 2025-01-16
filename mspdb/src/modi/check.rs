//! Code for checking invariants in Module Streams.

use super::ModiStreamData;
use crate::dbi::ModuleInfo;
use crate::diag::Diags;
use tracing::{error, trace_span};

/// Checks invariants of a Module Stream.
pub fn check_module_stream(
    diags: &mut Diags,
    module_index: usize,
    module: &ModuleInfo,
    module_stream: &ModiStreamData,
    names: &crate::names::NamesStream<Vec<u8>>,
    sources: &crate::dbi::sources::DbiSourcesSubstream<'_>,
) -> anyhow::Result<()> {
    let _span = trace_span!("check_module_stream").entered();

    let expected_stream_size: u32 = module.header().c11_byte_size.get()
        + module.header().c13_byte_size.get()
        + module.header().sym_byte_size.get();

    let module_stream_index = module.stream().unwrap();

    if module_stream.stream_data.len() < expected_stream_size as usize {
        error!(
            module_index,
            module_stream_index,
            expected_stream_size = expected_stream_size,
            actual_stream_size = module_stream.stream_data.len(),
            "module has substream sizes that exceed the actual size of the module stream."
        );
        return Ok(());
    }

    let c13_line_data = module_stream.c13_line_data_bytes();

    crate::lines::check::check_line_data(diags, module_index, names, sources, c13_line_data)?;

    Ok(())
}
