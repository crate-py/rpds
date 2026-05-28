fn main() {
    // macOS 15 runners use Apple's new linker (ld_prime), which reserves
    // far less Mach-O header padding than the old ld. Tools like Homebrew
    // run install_name_tool on installed dylibs, which needs spare header
    // space; without this flag the relink fails with "updated load commands
    // do not fit in the header". See crate-py/rpds#200, and pyca/cryptography
    // hit the same regression when its CI moved to macos-15.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-headerpad_max_install_names");
    }
}
