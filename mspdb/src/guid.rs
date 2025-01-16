//! Standard Windows type

use std::fmt::Debug;
use uuid::Uuid;
use zerocopy::{AsBytes, FromBytes, FromZeroes, Unaligned, LE, U16, U32};

/// Standard Windows type
#[repr(C)]
#[derive(Clone, Eq, PartialEq, AsBytes, FromBytes, FromZeroes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct GuidLe {
    pub data1: U32<LE>,
    pub data2: U16<LE>,
    pub data3: U16<LE>,
    pub data4: [u8; 8],
}

impl GuidLe {
    /// Convert the on-disk format to in-memory format.
    pub fn get(&self) -> Uuid {
        Uuid::from_fields(
            self.data1.get(),
            self.data2.get(),
            self.data3.get(),
            &self.data4,
        )
    }
}

impl From<&Uuid> for GuidLe {
    fn from(uuid: &Uuid) -> Self {
        let f = uuid.as_fields();
        GuidLe {
            data1: U32::new(f.0),
            data2: U16::new(f.1),
            data3: U16::new(f.2),
            data4: *f.3,
        }
    }
}
