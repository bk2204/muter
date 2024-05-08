extern crate autocfg;

fn main() {
    // Ideally we'd allow arbitrary byte paths here, but Rust's env! doesn't support that.  If this
    // becomes a problem, we can always percent-encode as a workaround.
    let sharedir = match (
        std::env::var("sharedir"),
        std::env::var("prefix"),
        std::env::var("CARGO_MANIFEST_DIR"),
    ) {
        (Ok(x), _, _) => x,
        (Err(_), Ok(x), _) => format!("{}/share", x),
        (Err(_), Err(_), Ok(x)) => x,
        (Err(_), Err(_), Err(_)) => "/usr/local/share".to_string(),
    };
    println!("cargo:rustc-env=sharedir={}", sharedir);
}
