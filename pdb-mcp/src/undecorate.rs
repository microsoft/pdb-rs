/// Attempts to undecorate (demangle) a symbol name.
///
/// Tries MSVC, Rust, and Itanium C++ schemes in order.
/// Returns `Some(demangled)` if successful, `None` if the name is not decorated.
pub fn try_undecorate(name: &str) -> Option<String> {
    // MSVC decorated names start with '?'
    if name.starts_with('?') {
        if let Ok(demangled) = msvc_demangler::demangle(name, msvc_demangler::DemangleFlags::llvm()) {
            return Some(demangled);
        }
    }

    // Rust v0 mangling starts with "_R"
    // Rust legacy mangling starts with "_ZN" (subset of Itanium)
    // Try rustc-demangle first since it handles both Rust schemes
    {
        let demangled = rustc_demangle::demangle(name);
        let demangled_str = format!("{demangled}");
        // rustc_demangle returns the input unchanged if it can't demangle
        if demangled_str != name {
            return Some(demangled_str);
        }
    }

    // Itanium C++ (Clang/GCC) starts with "_Z"
    if name.starts_with("_Z") {
        if let Ok(sym) = cpp_demangle::Symbol::new(name.as_bytes()) {
            if let Ok(demangled) = sym.demangle() {
                return Some(demangled);
            }
        }
    }

    None
}

/// Undecorate a name, returning the demangled form or the original if not decorated.
pub fn undecorate_or_original(name: &str) -> String {
    try_undecorate(name).unwrap_or_else(|| name.to_string())
}

/// Format a name showing both decorated and undecorated forms.
/// Returns "demangled (decorated)" if decoration was removed, or just the name unchanged.
pub fn format_with_undecoration(name: &str) -> String {
    match try_undecorate(name) {
        Some(demangled) => format!("{demangled} ({name})"),
        None => name.to_string(),
    }
}
