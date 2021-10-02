use diesel::r2d2::Builder;
use diesel::{connection::SimpleConnection, pg::PgConnection};
use diesel::{
    r2d2::{self, event as e, ConnectionManager, HandleEvent, Pool, PooledConnection},
    Connection,
};
use diesel::{sql_query, RunQueryDsl};
use postgres::config::{Config, Host};
use std::fmt::{self, Write};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, sync::RwLock};

use massbit::cheap_clone::CheapClone;
use massbit::constraint_violation;
use massbit::prelude::tokio::time::Instant;
use massbit::prelude::{tokio, Logger};
use massbit::util::timed_rw_lock::TimedMutex;
use massbit::{
    prelude::{
        anyhow::{self, anyhow, bail},
        crit, debug, error, info, o,
        tokio::sync::Semaphore,
        CancelGuard, CancelHandle, CancelToken as _, CancelableError, StoreError,
    },
    util::security::SafeDisplay,
};

use crate::{advisory_lock, catalog};
use crate::{Shard, PRIMARY_SHARD};

lazy_static::lazy_static! {
    // These environment variables should really be set through the
    // configuration file; especially for min_idle and idle_timeout, it's
    // likely that they should be configured differently for each pool

    static ref CONNECTION_TIMEOUT: Duration = {
        std::env::var("DATABASE_CONNECTION_TIMEOUT").ok().map(|s| Duration::from_millis(u64::from_str(&s).unwrap_or_else(|_| {
            panic!("DATABASE_CONNECTION_TIMEOUT must be a positive number, but is `{}`", s)
        }))).unwrap_or(Duration::from_secs(5))
    };
    static ref MIN_IDLE: Option<u32> = {
        std::env::var("DATABASE_CONNECTION_MIN_IDLE").ok().map(|s| u32::from_str(&s).unwrap_or_else(|_| {
           panic!("DATABASE_CONNECTION_MIN_IDLE must be a positive number but is `{}`", s)
        }))
    };
    static ref IDLE_TIMEOUT: Duration = {
        std::env::var("DATABASE_CONNECTION_IDLE_TIMEOUT").ok().map(|s| Duration::from_secs(u64::from_str(&s).unwrap_or_else(|_| {
            panic!("DATABASE_CONNECTION_IDLE_TIMEOUT must be a positive number, but is `{}`", s)
        }))).unwrap_or(Duration::from_secs(600))
    };
}

/// A pool goes through several states, and this enum tracks what state we
/// are in. When first created, the pool is in state `Created`; once we
/// successfully called `setup` on it, it moves to state `Ready`. If we
/// detect that the database is not available, the pool goes into state
/// `Unavailable` until we can confirm that we can connect to the database,
/// at which point it goes back to `Ready`
// `Unavailable` is not used in the code yet
#[allow(dead_code)]
enum PoolState {
    Created(Arc<PoolInner>),
    Unavailable(Arc<PoolInner>),
    Ready(Arc<PoolInner>),
}

#[derive(Clone)]
pub struct ConnectionPool {
    inner: Arc<TimedMutex<PoolState>>,
    logger: Logger,
}

/// The name of the pool, mostly for logging, and what purpose it serves.
/// The main pool will always be called `main`, and can be used for reading
/// and writing. Replica pools can only be used for reading, and don't
/// require any setup (migrations etc.)
pub enum PoolName {
    Main,
    Replica(String),
}

impl PoolName {
    fn as_str(&self) -> &str {
        match self {
            PoolName::Main => "main",
            PoolName::Replica(name) => name,
        }
    }

    fn is_replica(&self) -> bool {
        match self {
            PoolName::Main => false,
            PoolName::Replica(_) => true,
        }
    }
}

impl ConnectionPool {
    pub fn create(
        shard_name: &str,
        pool_name: PoolName,
        postgres_url: String,
        pool_size: u32,
        logger: &Logger,
    ) -> ConnectionPool {
        let pool = PoolInner::create(
            shard_name,
            pool_name.as_str(),
            postgres_url,
            pool_size,
            logger,
        );
        let pool_state = if pool_name.is_replica() {
            PoolState::Ready(Arc::new(pool))
        } else {
            PoolState::Created(Arc::new(pool))
        };
        ConnectionPool {
            inner: Arc::new(TimedMutex::new(pool_state, format!("pool-{}", shard_name))),
            logger: logger.clone(),
        }
    }

    /// Return a pool that is ready, i.e., connected to the database. If the
    /// pool has not been set up yet, call `setup`. If there are any errors
    /// or the pool is marked as unavailable, return
    /// `StoreError::DatabaseUnavailable`
    fn get_ready(&self) -> Result<Arc<PoolInner>, StoreError> {
        let mut guard = self.inner.lock();
        match &*guard {
            PoolState::Created(pool) => {
                let pool2 = pool.clone();
                *guard = PoolState::Ready(pool.clone());
                Ok(pool2)
            }
            PoolState::Unavailable(_) => Err(StoreError::DatabaseUnavailable),
            PoolState::Ready(pool) => Ok(pool.clone()),
        }
    }

    /// Execute a closure with a connection to the database.
    ///
    /// # API
    ///   The API of using a closure to bound the usage of the connection serves several
    ///   purposes:
    ///
    ///   * Moves blocking database access out of the `Future::poll`. Within
    ///     `Future::poll` (which includes all `async` methods) it is illegal to
    ///     perform a blocking operation. This includes all accesses to the
    ///     database, acquiring of locks, etc. Calling a blocking operation can
    ///     cause problems with `Future` combinators (including but not limited
    ///     to select, timeout, and FuturesUnordered) and problems with
    ///     executors/runtimes. This method moves the database work onto another
    ///     thread in a way which does not block `Future::poll`.
    ///
    ///   * Limit the total number of connections. Because the supplied closure
    ///     takes a reference, we know the scope of the usage of all entity
    ///     connections and can limit their use in a non-blocking way.
    ///
    /// # Cancellation
    ///   The normal pattern for futures in Rust is drop to cancel. Once we
    ///   spawn the database work in a thread though, this expectation no longer
    ///   holds because the spawned task is the independent of this future. So,
    ///   this method provides a cancel token which indicates that the `Future`
    ///   has been dropped. This isn't *quite* as good as drop on cancel,
    ///   because a drop on cancel can do things like cancel http requests that
    ///   are in flight, but checking for cancel periodically is a significant
    ///   improvement.
    ///
    ///   The implementation of the supplied closure should check for cancel
    ///   between every operation that is potentially blocking. This includes
    ///   any method which may interact with the database. The check can be
    ///   conveniently written as `token.check_cancel()?;`. It is low overhead
    ///   to check for cancel, so when in doubt it is better to have too many
    ///   checks than too few.
    ///
    /// # Panics:
    ///   * This task will panic if the supplied closure panics
    ///   * This task will panic if the supplied closure returns Err(Cancelled)
    ///     when the supplied cancel token is not cancelled.
    pub(crate) async fn with_conn<T: Send + 'static>(
        &self,
        f: impl 'static
            + Send
            + FnOnce(
                &PooledConnection<ConnectionManager<PgConnection>>,
                &CancelHandle,
            ) -> Result<T, CancelableError<StoreError>>,
    ) -> Result<T, StoreError> {
        let pool = self.get_ready()?;
        pool.with_conn(f).await
    }

    pub fn get(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, StoreError> {
        self.get_ready()?.get()
    }

    pub fn get_with_timeout_warning(
        &self,
        logger: &Logger,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, StoreError> {
        self.get_ready()?.get_with_timeout_warning(logger)
    }

    /// Check that we can connect to the database
    pub fn check(&self) -> bool {
        true
    }

    /// Setup the database for this pool. This includes configuring foreign
    /// data wrappers for cross-shard communication, and running any pending
    /// schema migrations for this database.
    ///
    /// # Panics
    ///
    /// If any errors happen during the migration, the process panics
    pub fn setup(&self) {
        let mut guard = self.inner.lock();
        match &*guard {
            PoolState::Created(pool) => {
                // If setup errors, we will try again later
                if pool.setup().is_ok() {
                    *guard = PoolState::Ready(pool.clone());
                }
            }
            PoolState::Unavailable(_) | PoolState::Ready(_) => { /* setup was previously done */ }
        }
    }
}

#[derive(Clone)]
pub struct PoolInner {
    logger: Logger,
    shard: Shard,
    pool: Pool<ConnectionManager<PgConnection>>,
    postgres_url: String,
}

impl PoolInner {
    pub fn create(
        shard_name: &str,
        pool_name: &str,
        postgres_url: String,
        pool_size: u32,
        logger: &Logger,
    ) -> PoolInner {
        let logger_store = logger.new(o!("component" => "Store"));
        let logger_pool = logger.new(o!("component" => "ConnectionPool"));
        // Connect to Postgres
        let conn_manager = ConnectionManager::new(postgres_url.clone());
        let builder: Builder<ConnectionManager<PgConnection>> = Pool::builder()
            .connection_timeout(*CONNECTION_TIMEOUT)
            .max_size(pool_size)
            .min_idle(*MIN_IDLE)
            .idle_timeout(Some(*IDLE_TIMEOUT));
        let pool = builder.build_unchecked(conn_manager);

        info!(logger_store, "Pool successfully connected to Postgres");

        PoolInner {
            logger: logger_pool,
            shard: Shard::new(shard_name.to_string())
                .expect("shard_name is a valid name for a shard"),
            postgres_url: postgres_url.clone(),
            pool,
        }
    }

    /// Execute a closure with a connection to the database.
    ///
    /// # API
    ///   The API of using a closure to bound the usage of the connection serves several
    ///   purposes:
    ///
    ///   * Moves blocking database access out of the `Future::poll`. Within
    ///     `Future::poll` (which includes all `async` methods) it is illegal to
    ///     perform a blocking operation. This includes all accesses to the
    ///     database, acquiring of locks, etc. Calling a blocking operation can
    ///     cause problems with `Future` combinators (including but not limited
    ///     to select, timeout, and FuturesUnordered) and problems with
    ///     executors/runtimes. This method moves the database work onto another
    ///     thread in a way which does not block `Future::poll`.
    ///
    ///   * Limit the total number of connections. Because the supplied closure
    ///     takes a reference, we know the scope of the usage of all entity
    ///     connections and can limit their use in a non-blocking way.
    ///
    /// # Cancellation
    ///   The normal pattern for futures in Rust is drop to cancel. Once we
    ///   spawn the database work in a thread though, this expectation no longer
    ///   holds because the spawned task is the independent of this future. So,
    ///   this method provides a cancel token which indicates that the `Future`
    ///   has been dropped. This isn't *quite* as good as drop on cancel,
    ///   because a drop on cancel can do things like cancel http requests that
    ///   are in flight, but checking for cancel periodically is a significant
    ///   improvement.
    ///
    ///   The implementation of the supplied closure should check for cancel
    ///   between every operation that is potentially blocking. This includes
    ///   any method which may interact with the database. The check can be
    ///   conveniently written as `token.check_cancel()?;`. It is low overhead
    ///   to check for cancel, so when in doubt it is better to have too many
    ///   checks than too few.
    ///
    /// # Panics:
    ///   * This task will panic if the supplied closure panics
    ///   * This task will panic if the supplied closure returns Err(Cancelled)
    ///     when the supplied cancel token is not cancelled.
    pub(crate) async fn with_conn<T: Send + 'static>(
        &self,
        f: impl 'static
            + Send
            + FnOnce(
                &PooledConnection<ConnectionManager<PgConnection>>,
                &CancelHandle,
            ) -> Result<T, CancelableError<StoreError>>,
    ) -> Result<T, StoreError> {
        let pool = self.clone();

        let cancel_guard = CancelGuard::new();
        let cancel_handle = cancel_guard.handle();

        let result = massbit::spawn_blocking_allow_panic(move || {
            // It is possible time has passed between scheduling on the
            // threadpool and being executed. Time to check for cancel.
            cancel_handle.check_cancel()?;

            // A failure to establish a connection is propagated as though the
            // closure failed.
            let conn = pool
                .get()
                .map_err(|e| CancelableError::Error(StoreError::Unknown(e.into())))?;

            // It is possible time has passed while establishing a connection.
            // Time to check for cancel.
            cancel_handle.check_cancel()?;

            f(&conn, &cancel_handle)
        })
        .await
        .unwrap(); // Propagate panics, though there shouldn't be any.

        drop(cancel_guard);

        // Finding cancel isn't technically unreachable, since there is nothing
        // stopping the supplied closure from returning Canceled even if the
        // supplied handle wasn't canceled. That would be very unexpected, the
        // doc comment for this function says we will panic in this scenario.
        match result {
            Ok(t) => Ok(t),
            Err(CancelableError::Error(e)) => Err(e),
            Err(CancelableError::Cancel) => panic!("The closure supplied to with_entity_conn must not return Err(Canceled) unless the supplied token was canceled."),
        }
    }

    pub fn get(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, StoreError> {
        self.pool.get().map_err(|e| StoreError::Unknown(e.into()))
    }

    pub fn get_with_timeout_warning(
        &self,
        logger: &Logger,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, StoreError> {
        loop {
            match self.pool.get_timeout(*CONNECTION_TIMEOUT) {
                Ok(conn) => return Ok(conn),
                Err(e) => error!(logger, "Error checking out connection, retrying";
                   "error" => e.to_string(),
                ),
            }
        }
    }

    /// Check that we can connect to the database
    pub fn check(&self) -> bool {
        self.pool
            .get()
            .ok()
            .map(|conn| sql_query("select 1").execute(&conn).is_ok())
            .unwrap_or(false)
    }

    /// Setup the database for this pool. This includes configuring foreign
    /// data wrappers for cross-shard communication, and running any pending
    /// schema migrations for this database.
    ///
    /// Returns `StoreError::DatabaseUnavailable` if we can't connect to the
    /// database. Any other error causes a panic.
    ///
    /// # Panics
    ///
    /// If any errors happen during the migration, the process panics
    pub fn setup(&self) -> Result<(), StoreError> {
        fn die(logger: &Logger, msg: &'static str, err: &dyn std::fmt::Display) -> ! {
            crit!(logger, "{}", msg; "error" => err.to_string());
            panic!("{}: {}", msg, err);
        }

        let pool = self.clone();
        let conn = self.get().map_err(|_| StoreError::DatabaseUnavailable)?;

        advisory_lock::lock_migration(&conn)
            .unwrap_or_else(|err| die(&pool.logger, "failed to get migration lock", &err));
        let result = migrate_schema(&pool.logger, &conn);
        advisory_lock::unlock_migration(&conn).unwrap_or_else(|err| {
            die(&pool.logger, "failed to release migration lock", &err);
        });
        result.unwrap_or_else(|err| die(&pool.logger, "migrations failed", &err));
        Ok(())
    }
}

embed_migrations!("./migrations");

/// Run all schema migrations.
///
/// When multiple `graph-node` processes start up at the same time, we ensure
/// that they do not run migrations in parallel by using `blocking_conn` to
/// serialize them. The `conn` is used to run the actual migration.
fn migrate_schema(logger: &Logger, conn: &PgConnection) -> Result<(), StoreError> {
    // Collect migration logging output
    let mut output = vec![];

    info!(logger, "Running migrations");
    let result = embedded_migrations::run_with_output(conn, &mut output);
    info!(logger, "Migrations finished");

    // If there was any migration output, log it now
    let msg = String::from_utf8(output).unwrap_or_else(|_| String::from("<unreadable>"));
    let msg = msg.trim();
    let has_output = !msg.is_empty();
    if has_output {
        let msg = msg.replace('\n', " ");
        if let Err(e) = result {
            error!(logger, "Postgres migration error"; "output" => msg);
            return Err(StoreError::Unknown(e.into()));
        } else {
            debug!(logger, "Postgres migration output"; "output" => msg);
        }
    }

    if has_output {
        // We take getting output as a signal that a migration was actually
        // run, which is not easy to tell from the Diesel API, and reset the
        // query statistics since a schema change makes them not all that
        // useful. An error here is not serious and can be ignored.
        conn.batch_execute("select pg_stat_statements_reset()").ok();
    }

    Ok(())
}
