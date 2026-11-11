//! `MadeFrom` and `Numbered` (the review, BUG-10): a value is known by
//! itself, never by where it is in memory. Nothing here leans on where
//! the allocator puts a value: with the value itself remembered, where
//! it is can't matter.

use super::*;

#[test]
fn only_the_very_value_it_was_made_from_is_it() {
    let sheet = Arc::new(vec![1u8, 2, 3]);
    let made = MadeFrom::new(&sheet);
    assert!(made.is(&sheet));
    assert!(
        made.is(&Arc::clone(&sheet)),
        "another handle on the same value"
    );
    assert!(made.clone().is(&sheet));
    // Another value with the same in it is another value.
    let alike = Arc::new(vec![1u8, 2, 3]);
    assert_eq!(*alike, *sheet);
    assert!(!made.is(&alike));
    assert!(!MadeFrom::new(&alike).is(&sheet));
}

/// The bug: a value let go, and the next one made put where it was. Many
/// are made here one after the other, each as big as the one let go, so
/// some would land there if anything could; none is ever taken for it.
#[test]
fn a_value_made_after_it_was_let_go_is_never_it() {
    let first = Arc::new([7u64; 16]);
    let made = MadeFrom::new(&first);
    drop(first);
    for _ in 0..1000 {
        let next = Arc::new([7u64; 16]);
        assert!(!made.is(&next));
    }
    // And remembering a value doesn't keep it: it was let go at once.
    let kept = Arc::new(vec![0u8; 1 << 20]);
    let weak = Arc::downgrade(&kept);
    let made = MadeFrom::new(&kept);
    drop(kept);
    assert!(weak.upgrade().is_none(), "its megabyte isn't held on to");
    assert!(!made.is(&Arc::new(vec![0u8; 1 << 20])));
}

#[test]
fn made_from_nothing_is_the_same_only_as_nothing() {
    let program = Arc::new("program".to_string());
    let made = MadeFrom::new(&program);
    assert!(MadeFrom::<String>::same(None, None));
    assert!(MadeFrom::same(Some(&made), Some(&program)));
    assert!(!MadeFrom::same(Some(&made), None));
    assert!(!MadeFrom::same(None, Some(&program)));
    let other = Arc::new("program".to_string());
    assert!(!MadeFrom::same(Some(&made), Some(&other)));
}

#[test]
fn each_value_has_a_number_no_other_value_ever_has() {
    let mut numbers = Numbered::default();
    let (a, b) = (Arc::new(1u8), Arc::new(1u8));
    let (na, nb) = (numbers.number(&a), numbers.number(&b));
    assert!(na != 0 && nb != 0 && na != nb, "alike, but two values");
    assert_eq!(numbers.number(&a), na, "the same for as long as it lasts");
    assert_eq!(numbers.number(&Arc::clone(&b)), nb);
    // Tidying forgets only what's gone: a value keeps its number whether
    // or not it was asked about meanwhile.
    numbers.forget_gone();
    numbers.forget_gone();
    assert_eq!((numbers.number(&a), numbers.number(&b)), (na, nb));
    assert_eq!(numbers.known.len(), 2);
    // A value let go and others made after it: none gets its number, and
    // the list holds only what there is.
    let mut seen = vec![na, nb];
    drop(a);
    for _ in 0..1000 {
        let next = Arc::new(1u8);
        let n = numbers.number(&next);
        assert!(!seen.contains(&n));
        seen.push(n);
        drop(next);
        numbers.forget_gone();
        assert_eq!(numbers.known.len(), 1, "only `b` is left");
    }
    assert_eq!(numbers.number(&b), nb);
    // Nor does another list of numbers ever give one of these.
    let mut others = Numbered::default();
    assert!(!seen.contains(&others.number(&b)));
}
