use slog::Logger;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

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
impl CheapClone for Logger {}

// Pool is implemented as a newtype over Arc,
// So it is CheapClone.
impl<M: diesel::r2d2::ManageConnection> CheapClone for diesel::r2d2::Pool<M> {}

impl<F: Future> CheapClone for futures03::future::Shared<F> {}
