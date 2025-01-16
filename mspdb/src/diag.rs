//! Diagnostics (warnings and errors)
#![allow(missing_docs)]

use bstr::BStr;
use dump_utils::HexDump;
use log::trace;

pub struct Diags {
    pub num_errors: u32,
    pub num_warnings: u32,
    pub max_errors: u32,
    pub diags: Vec<Diag>,
}

impl Default for Diags {
    fn default() -> Self {
        Self::new()
    }
}

impl Diags {
    pub fn new() -> Self {
        Diags {
            num_errors: 0,
            num_warnings: 0,
            max_errors: 20,
            diags: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        self.num_errors != 0
    }

    pub fn has_warnings(&self) -> bool {
        self.num_warnings != 0
    }

    pub fn wants_error(&self) -> bool {
        self.num_errors < self.max_errors
    }

    pub fn wants_warning(&self) -> bool {
        self.num_errors < self.max_errors && self.num_warnings < 100
    }

    pub fn error_opt<S: Into<String>>(&mut self, msg: S) -> Option<&mut Diag> {
        if self.wants_error() {
            Some(self.error_with(msg))
        } else {
            None
        }
    }

    pub fn error<S: Into<String>>(&mut self, msg: S) {
        if !self.wants_error() {
            return;
        }

        self.num_errors += 1;
        let msg: String = msg.into();
        trace!("error : {}", msg);
        self.diags.push(Diag {
            message: msg.to_string(),
            is_error: true,
            refs: Vec::new(),
        });
    }

    pub fn error_with<S: Into<String>>(&mut self, msg: S) -> &mut Diag {
        self.num_errors += 1;
        let msg: String = msg.into();
        trace!("error : {}", msg);
        self.diags.push(Diag {
            message: msg.to_string(),
            is_error: true,
            refs: Vec::new(),
        });
        self.diags.last_mut().unwrap()
    }

    pub fn warning<S: Into<String>>(&mut self, msg: S) -> Option<&mut Diag> {
        if !self.wants_warning() {
            return None;
        }

        self.num_warnings += 1;
        let msg: String = msg.into();
        trace!("warning : {}", msg);
        self.diags.push(Diag {
            message: msg.to_string(),
            is_error: false,
            refs: Vec::new(),
        });
        self.diags.last_mut()
    }
}

pub struct Diag {
    pub message: String,
    pub is_error: bool,
    pub refs: Vec<DiagRef>,
}

pub enum DiagRef {
    Stream(u32),
    StreamAt(u32, u32), // stream, byte offset
    Module { module_index: u32, name: String },
    Bytes(Vec<u8>, usize),
}

impl Diag {
    pub fn module(&mut self, module_index: u32, name: &BStr) -> &mut Self {
        trace!("    at module # {module_index} - {name}");
        self.refs.push(DiagRef::Module {
            module_index,
            name: name.to_string(),
        });
        self
    }

    pub fn stream(&mut self, stream: u32) -> &mut Self {
        trace!("    at stream # {}", stream);
        self.refs.push(DiagRef::Stream(stream));
        self
    }

    pub fn stream_at(&mut self, stream: u32, offset: u32) -> &mut Self {
        trace!("    at stream # {} offset 0x{:08x}", stream, offset);
        self.refs.push(DiagRef::StreamAt(stream, offset));
        self
    }

    pub fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        trace!("    data:");
        trace!("{:?}", HexDump::new(bytes).max(256));
        self.refs.push(DiagRef::Bytes(bytes.to_vec(), 0));
        self
    }

    pub fn bytes_at(&mut self, offset: usize, bytes: &[u8]) -> &mut Self {
        trace!("    data:");
        trace!("{:?}", HexDump::new(bytes).max(256));
        self.refs.push(DiagRef::Bytes(bytes.to_vec(), offset));
        self
    }

    pub fn err<E: std::fmt::Display>(&mut self, e: E) -> &mut Self {
        self.message.push('\n');
        self.message.push_str(&e.to_string());
        self
    }
}

impl std::fmt::Display for Diags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for diag in self.diags.iter() {
            write!(f, "{}", diag)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for Diag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_error {
            write!(f, "error: ")?;
        } else {
            write!(f, "warning: ")?;
        }
        writeln!(f, "{}", self.message)?;

        for r in self.refs.iter() {
            match r {
                DiagRef::Bytes(data, offset) => {
                    writeln!(
                        f,
                        "  bytes:\n{:?}",
                        HexDump::new(data).at(*offset).max(0x200)
                    )?;
                }
                DiagRef::Module { module_index, name } => {
                    writeln!(f, "  module: #{module_index} - {name}")?;
                }
                DiagRef::Stream(stream) => {
                    writeln!(f, "  stream: {stream}")?;
                }
                DiagRef::StreamAt(stream, offset) => {
                    writeln!(f, "  stream: {stream} at offset 0x{offset:x}")?;
                }
            }
        }

        Ok(())
    }
}
