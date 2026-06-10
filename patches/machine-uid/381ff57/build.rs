fn main() {
    println!("cargo:rerun-if-changed=src");
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=Kernel32");
        cc::Build::new().file("src/win.cpp").compile("machine-uid");
    }
}
