use diesel::r2d2;
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use massbit_common::prelude::log;
use monitor::opt;
use monitor::service::Monitor;
use std::future::pending;
use std::sync::Arc;
use structopt::StructOpt;

#[tokio::main]
async fn main() {
    let opt = opt::Opt::from_args();
    log::info!("{:?}", &opt);
    let manager = ConnectionManager::<PgConnection>::new(opt.database_url);
    let connection_pool = r2d2::Pool::builder()
        .max_size(opt.pool_size)
        .build(manager)
        .expect("Can not create connection pool");
    let monitor = Monitor::new(Arc::new(connection_pool), opt.monitor_period);
    monitor.start();
    pending::<()>().await;
}
