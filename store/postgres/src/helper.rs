use diesel::r2d2::ConnectionManager;
use diesel::{r2d2, Connection};

pub fn create_r2d2_connection_pool<T: 'static + Connection>(
    db_url: &str,
    pool_size: u32,
) -> r2d2::Pool<ConnectionManager<T>> {
    let manager = ConnectionManager::<T>::new(db_url);
    r2d2::Pool::builder()
        .max_size(pool_size)
        .build(manager)
        .expect("Can not create connection pool")
}
