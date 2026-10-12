// Placeholder entry point; commands arrive with the `static-build` change.
fn main() {
    println!("hermetic {}", env!("CARGO_PKG_VERSION"));
}
