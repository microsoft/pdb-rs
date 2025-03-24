//! Utilities for working with filesystem paths

use std::path::Path;

/// Tests whether `container_path` is equal to `nested_path` or is an ancestor of `nested_path`.
pub fn path_contains(container_path: &str, nested_path: &str) -> bool {
    let c_path = Path::new(container_path);
    let n_path = Path::new(nested_path);

    if c_path.is_absolute() != n_path.is_absolute() {
        return false;
    }

    let mut ci = c_path.components();
    let mut ni = n_path.components();

    loop {
        match (ci.next(), ni.next()) {
            (Some(ce), Some(ne)) => {
                // Ignore case, because Windows.
                if !ce.as_os_str().eq_ignore_ascii_case(ne.as_os_str()) {
                    return false;
                }
            }

            // We ran out of nested elements, but still have more container elements. Not a match.
            (Some(_), None) => return false,

            // We ran out of container elements, so it's a match.
            (None, _) => return true,
        }
    }
}

#[test]
#[cfg(windows)]
fn test_path_contains() {
    assert!(!path_contains(r"d:\src", r"foo.c"));

    assert!(path_contains(r"d:\src", r"d:\src\foo.c"));
    assert!(path_contains(r"d:\src", r"D:\SRC\\foo.c"));
    assert!(path_contains(r"d:\src\", r"d:\src"));
    assert!(path_contains(r"d:\src", r"d:\src\"));

    // negative cases
    assert!(!path_contains(r"d:\src", r"e:\src\foo.c"));
    assert!(!path_contains(r"d:\src", r"d:\bar"));
}
