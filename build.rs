fn main() {
    println!("cargo:rerun-if-changed=src/grammar.pest");
}
