macro_rules! debug {
    ($($t:tt)*) => {
        #[cfg(test)]
        {
            let msg = format!($($t)*);
            println!("{}({}): {}", file!(), line!(), msg);
        }
    }
}
