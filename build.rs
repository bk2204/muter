extern crate autocfg;

fn main() {
    let ac = autocfg::new();
    ac.emit_rustc_version(1, 37);
    ac.emit_trait_cfg(
        "std::ops::RangeBounds<std::ops::Range<usize>>",
        "has_range_bounds",
    );

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
