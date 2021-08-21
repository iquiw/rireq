#[cfg(windows)]
fn main() {
    println!("cargo:rustc-link-lib=dylib=ntdll");
    println!("cargo:rustc-link-lib=dylib=user32");
}
#[cfg(not(windows))]
fn main() {
}
