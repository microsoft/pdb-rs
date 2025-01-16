
pub enum OpenPdbError {
    IsPortablePdb,
    IO(std::error::Error),
}
