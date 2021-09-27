use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

/// Things that are fast to clone in the context of an application such as Graph Node
///
/// The purpose of this API is to reduce the number of calls to .clone() which need to
/// be audited for performance.
///
/// As a rule of thumb, only constant-time Clone impls should also implement CheapClone.
/// Eg:
///    ✔ Arc<T>
///    ✗ Vec<T>
///    ✔ u128
///    ✗ String
pub trait CheapClone: Clone {
    #[inline]
    fn cheap_clone(&self) -> Self {
        self.clone()
    }
}

impl<T: ?Sized> CheapClone for Rc<T> {}
impl<T: ?Sized> CheapClone for Arc<T> {}
impl<T: ?Sized + CheapClone> CheapClone for Box<T> {}
impl<T: ?Sized + CheapClone> CheapClone for std::pin::Pin<T> {}
impl<T: CheapClone> CheapClone for Option<T> {}
impl<F: Future> CheapClone for futures03::future::Shared<F> {}
