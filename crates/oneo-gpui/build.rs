fn main() {
    println!("cargo:rerun-if-changed=examples/locales");
    println!("cargo:rerun-if-changed=locales");
}
