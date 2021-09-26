use massbit_common::prelude::r2d2;
use massbit_common::prelude::r2d2_diesel::ConnectionManager;
use massbit_common::prelude::diesel::Connection;
const MAX_POOL_SIZE : u32 = 10;
pub fn create_r2d2_connection_pool<T:'static + Connection>(db_url: &str) -> r2d2::Pool<ConnectionManager<T>> {
    let manager = ConnectionManager::<T>::new(db_url);
    r2d2::Pool::builder().max_size(MAX_POOL_SIZE).build(manager).expect("Can not create connection pool")
}
