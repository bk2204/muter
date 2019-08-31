extern crate autocfg;

fn main() {
    let ac = autocfg::new();
    ac.emit_rustc_version(1, 37);
    ac.emit_trait_cfg(
        "std::ops::RangeBounds<std::ops::Range<usize>>",
        "has_range_bounds",
    );
}
