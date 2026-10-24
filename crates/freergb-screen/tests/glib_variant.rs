#![cfg(target_os = "linux")]
//! RUSTSEC-2024-0429 regression. Run optimized: the original immutable
//! out-pointer may be optimized into a null dereference in release builds.
use glib::variant::ToVariant;

#[test]
fn string_variant_iterator_all_affected_operations_and_boundaries() {
    let value = ["first", "Grüße", "last"].to_variant();
    assert_eq!(
        value.array_iter_str().unwrap().collect::<Vec<_>>(),
        ["first", "Grüße", "last"]
    );
    assert_eq!(value.array_iter_str().unwrap().next(), Some("first"));
    assert_eq!(value.array_iter_str().unwrap().nth(1), Some("Grüße"));
    assert_eq!(value.array_iter_str().unwrap().last(), Some("last"));
    assert_eq!(value.array_iter_str().unwrap().next_back(), Some("last"));
    assert_eq!(value.array_iter_str().unwrap().nth_back(1), Some("Grüße"));
    let mut mixed = value.array_iter_str().unwrap();
    assert_eq!(mixed.next(), Some("first"));
    assert_eq!(mixed.next_back(), Some("last"));
    assert_eq!(mixed.next(), Some("Grüße"));
    assert_eq!(mixed.next(), None);
    assert_eq!(mixed.next_back(), None);
    let empty = Vec::<&str>::new().to_variant();
    assert_eq!(empty.array_iter_str().unwrap().next(), None);
}
