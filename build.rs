fn main() {
    // Without this, Cargo only reruns the build/link step when a .rs file
    // changes -- an assets/-only edit (e.g. puzzle-set JSON) would leave a
    // stale wasm32-unknown-emscripten build, since --preload-file bakes
    // assets/ into the .data blob at link time. The native build isn't
    // affected (it reads assets/ from disk at runtime), but this is cheap
    // enough to apply unconditionally.
    println!("cargo:rerun-if-changed=assets");
}
