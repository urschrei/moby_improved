//! Generates the Swift bindings, header and module map for `moby_ffi`.

fn main() {
    uniffi::uniffi_bindgen_swift();
}
