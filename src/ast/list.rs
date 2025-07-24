use super::value::Value;

/// An iterator over a Sutra list (a `ConsCell` chain).
///
/// This struct is created by the `try_into_iter` method on `Value`.
/// It iterates over the `car` of each `ConsCell` until it encounters
/// a `cdr` that is not a `Cons` value (typically `Value::Nil`).
pub struct ListIter {
    current: Value,
}

impl ListIter {
    /// Creates a new iterator starting from the given `Value`.
    pub fn new(value: Value) -> Self {
        ListIter { current: value }
    }
}

impl Iterator for ListIter {
    type Item = Value;

    /// Advances the iterator and returns the next value.
    ///
    /// The iterator yields the `car` of the current `ConsCell` and then
    /// moves to the `cdr`. The iteration stops when the current value
    /// is not a `ConsCell`. For a proper list, this will be `Value::Nil`.
    fn next(&mut self) -> Option<Self::Item> {
        match &self.current {
            Value::Cons(cell) => {
                let car = cell.car.clone();
                // Move to the next link in the chain.
                self.current = cell.cdr.clone();
                Some(car)
            }
            // If the current value is not a Cons cell, the list has ended.
            _ => None,
        }
    }
}

impl Value {
    /// Attempts to create an iterator over a `Value`.
    ///
    /// If the `Value` is a `Cons` or `Nil`, it returns a `ListIter`.
    /// This is the primary way to traverse list structures.
    pub fn try_into_iter(self) -> ListIter {
        ListIter::new(self)
    }
}
