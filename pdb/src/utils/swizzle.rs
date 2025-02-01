//! Traits and support for in-place byte-order swizzling

/// Defines the behavior of converting from the host byte order to specific external byte orders
/// (LE and BE).
pub trait Swizzle {
    /// Converts values within this value from LE order to host order.
    /// On LE architectures, this does nothing.
    fn le_to_host(&mut self);
}

macro_rules! int_swizzle {
    ($t:ty) => {
        impl Swizzle for $t {
            fn le_to_host(&mut self) {
                if cfg!(target_endian = "big") {
                    *self = Self::from_le(*self);
                }
            }
        }
    };
}

int_swizzle!(u16);
int_swizzle!(u32);
int_swizzle!(u64);
int_swizzle!(u128);

int_swizzle!(i16);
int_swizzle!(i32);
int_swizzle!(i64);
int_swizzle!(i128);

impl Swizzle for u8 {
    fn le_to_host(&mut self) {}
}

impl Swizzle for i8 {
    fn le_to_host(&mut self) {}
}

impl<T: Swizzle> Swizzle for [T] {
    fn le_to_host(&mut self) {
        for i in self.iter_mut() {
            i.le_to_host();
        }
    }
}
