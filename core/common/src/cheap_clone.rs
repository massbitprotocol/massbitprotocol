use tonic::transport::Channel;

pub trait CheapClone: Clone {
    #[inline]
    fn cheap_clone(&self) -> Self {
        self.clone()
    }
}
impl CheapClone for Channel {}
