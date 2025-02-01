pub(crate) fn show_comp_rate(description: &str, before: u64, after: u64) {
    if before == 0 {
        // We don't divide by zero around here.
        println!(
            "    {:-30} : {:8} -> {:8}",
            description,
            friendly::bytes(before),
            friendly::bytes(after)
        );
    } else {
        let percent = (before as f64 - after as f64) / (before as f64) * 100.0;
        println!(
            "    {:-30} : {:8} -> {:8} {:2.1} %",
            description,
            friendly::bytes(before),
            friendly::bytes(after),
            percent
        );
    }
}
