//! Read-only: prints drive temperatures readable without administrator rights.
fn main() {
    for drive in freergb_storage_temp::read_all() {
        println!("{drive:?}");
    }
}
