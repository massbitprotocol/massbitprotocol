use super::models::Indexer;
use uuid::Uuid;
impl Indexer {
    ///Always to call this function to create indexer for init hash value
    pub fn new() -> Self {
        let mut indexer = Indexer::default();
        indexer.hash = Uuid::new_v4().to_string().replace("-", "");
        indexer
    }
}
