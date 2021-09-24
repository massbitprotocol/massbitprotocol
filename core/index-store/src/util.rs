use massbit_common::prelude::r2d2;
use massbit_common::prelude::r2d2_diesel::ConnectionManager;
use massbit_common::prelude::diesel::Connection;

pub fn create_r2d2_connection_pool<T:'static + Connection>(db_url: &str) -> r2d2::Pool<ConnectionManager<T>> {
    let manager = ConnectionManager::<T>::new(db_url);
    r2d2::Pool::builder().build(manager).expect("Can not create connection pool")
}
