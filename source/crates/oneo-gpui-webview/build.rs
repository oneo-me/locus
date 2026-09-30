fn main() {
    println!("cargo:rerun-if-changed=src/host.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let deployment_target =
            std::env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "13.0".to_owned());
        cc::Build::new()
            .file("src/host.m")
            .flag("-fobjc-arc")
            .flag(format!("-mmacosx-version-min={deployment_target}"))
            .flag("-fmodules")
            .compile("oneo_webview_host");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=WebKit");
        println!("cargo:rustc-link-lib=framework=QuartzCore");
    }
}
