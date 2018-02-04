extern crate cc;

fn main() {
    cc::Build::new()
        .cpp(true)
        .file("src/foreign/fsa.cpp")
        .include("src/foreign/include")
        .cpp_link_stdlib(None)
        .try_compile("libfsa.a")
        .expect("Building of C bindings for OpenFst failed. Please make sure that OpenFst is installed.");
}
