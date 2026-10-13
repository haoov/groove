//! A set that holds a thing or not, turned the other way.

use std::collections::BTreeSet;

/// `one` taken out of `set` when it holds it, else put in.
pub fn toggle<T: Ord>(set: &mut BTreeSet<T>, one: T) {
    if !set.remove(&one) {
        set.insert(one);
    }
}
