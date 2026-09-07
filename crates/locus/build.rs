fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && std::env::var("PROFILE").as_deref() == Ok("release")
    {
        println!("cargo::rustc-link-arg-bin=locus=-Wl,-no_exported_symbols");
    }
}
