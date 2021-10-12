mod common;
use anyhow::Context;
use common::docker::{pull_images, DockerTestClient, TestContainerService};
use common::helpers::{
    basename, get_unique_postgres_counter, make_ipfs_uri, make_postgres_uri, pretty_output,
    IndexerManagerPorts, MappedPorts,
};
use futures::StreamExt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

const DEFAULT_N_CONCURRENT_TESTS: usize = 15;

lazy_static::lazy_static! {
    static ref IPFS_HARD_WAIT_SECONDS: Option<u64> =
        parse_numeric_environment_variable("TESTS_IPFS_HARD_WAIT_SECONDS");
    static ref POSTGRES_HARD_WAIT_SECONDS: Option<u64> =
        parse_numeric_environment_variable("TESTS_POSTGRES_HARD_WAIT_SECONDS");
}

/// All integration tests subdirectories to run
pub const INTEGRATION_TESTS_DIRECTORIES: [&str; 1] = ["api-version-v0-0-4"];

/// Contains all information a test command needs
#[derive(Debug)]
struct IntegrationTestSetup {
    postgres_uri: String,
    ipfs_uri: String,
    indexer_manager_ports: IndexerManagerPorts,
    indexer_manager_bin: Arc<PathBuf>,
    test_directory: PathBuf,
}

impl IntegrationTestSetup {
    fn test_name(&self) -> String {
        basename(&self.test_directory)
    }

    fn indexer_manager_uri(&self) -> String {
        let ws_port = self.indexer_manager_ports.json_rpc_port;
        format!("http://localhost:{}/", ws_port)
    }
}

/// Info about a finished test command
#[derive(Debug)]
struct TestCommandResults {
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug)]
struct StdIO {
    stdout: Option<String>,
    stderr: Option<String>,
}

impl std::fmt::Display for StdIO {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref stdout) = self.stdout {
            write!(f, "{}", stdout)?;
        }
        if let Some(ref stderr) = self.stderr {
            write!(f, "{}", stderr)?
        }
        Ok(())
    }
}

// The results of a finished integration test
#[derive(Debug)]
struct IntegrationTestResult {
    test_setup: IntegrationTestSetup,
    test_command_results: TestCommandResults,
    indexer_manager_stdio: StdIO,
}

impl IntegrationTestResult {
    fn print_outcome(&self) {
        let status = match self.test_command_results.success {
            true => "SUCCESS",
            false => "FAILURE",
        };
        println!("- Test: {}: {}", status, self.test_setup.test_name())
    }

    fn print_failure(&self) {
        if self.test_command_results.success {
            return;
        }
        let test_name = self.test_setup.test_name();
        println!("=============");
        println!("\nFailed test: {}", test_name);
        println!("-------------");
        println!("{:#?}", self.test_setup);
        println!("-------------");
        println!("\nFailed test command output:");
        println!("---------------------------");
        println!("{}", self.test_command_results.stdout);
        println!("{}", self.test_command_results.stderr);
        println!("--------------------------");
        println!("indexer-manager command output:");
        println!("--------------------------");
        println!("{}", self.indexer_manager_stdio);
    }
}

/// The main test entrypoint
#[tokio::test]
async fn parallel_integration_tests() -> anyhow::Result<()> {
    // use a environment variable for limiting the number of concurrent tests
    let n_parallel_tests: usize = std::env::var("N_CONCURRENT_TESTS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(DEFAULT_N_CONCURRENT_TESTS);

    let current_working_directory =
        std::env::current_dir().context("failed to identify working directory")?;
    let integration_tests_root_directory = current_working_directory.join("integration-tests");

    // pull required docker images
    pull_images().await;

    let test_directories = INTEGRATION_TESTS_DIRECTORIES
        .iter()
        .map(|ref p| integration_tests_root_directory.join(PathBuf::from(p)))
        .collect::<Vec<PathBuf>>();

    // Show discovered tests
    println!("Found {} integration tests:", test_directories.len());
    for dir in &test_directories {
        println!("  - {}", basename(dir));
    }

    // start docker containers for Postgres and IPFS and wait for them to be ready
    let postgres = Arc::new(
        DockerTestClient::start(TestContainerService::Postgres)
            .await
            .context("failed to start container service for Postgres.")?,
    );
    postgres
        .wait_for_message(
            b"database system is ready to accept connections",
            &*POSTGRES_HARD_WAIT_SECONDS,
        )
        .await
        .context("failed to wait for Postgres container to be ready to accept connections")?;

    let ipfs = DockerTestClient::start(TestContainerService::Ipfs)
        .await
        .context("failed to start container service for IPFS.")?;
    ipfs.wait_for_message(b"Daemon is ready", &*IPFS_HARD_WAIT_SECONDS)
        .await
        .context("failed to wait for Ipfs container to be ready to accept connections")?;

    let postgres_ports = Arc::new(
        postgres
            .exposed_ports()
            .await
            .context("failed to obtain exposed ports for the Postgres container")?,
    );
    let ipfs_ports = Arc::new(
        ipfs.exposed_ports()
            .await
            .context("failed to obtain exposed ports for the IPFS container")?,
    );

    let indexer_manger =
        Arc::new(fs::canonicalize("../target/debug/manager").context(
            "failed to infer `indexer-manager` program location. (Was it built already?)",
        )?);

    // run tests
    let mut test_results = Vec::new();

    let mut stream = tokio_stream::iter(test_directories)
        .map(|dir| {
            run_integration_test(
                dir.clone(),
                postgres.clone(),
                postgres_ports.clone(),
                ipfs_ports.clone(),
                indexer_manger.clone(),
            )
        })
        .buffered(n_parallel_tests);

    let mut failed = false;
    while let Some(test_result) = stream.next().await {
        let test_result = test_result?;
        if !test_result.test_command_results.success {
            failed = true;
        }
        test_results.push(test_result);
    }

    // Stop containers.
    postgres
        .stop()
        .await
        .context("failed to stop container service for Postgres")?;
    ipfs.stop()
        .await
        .context("failed to stop container service for IPFS")?;

    // print failures
    for failed_test in test_results
        .iter()
        .filter(|t| !t.test_command_results.success)
    {
        failed_test.print_failure()
    }

    // print test result summary
    println!("\nTest results:");
    for test_result in &test_results {
        test_result.print_outcome()
    }

    if failed {
        Err(anyhow::anyhow!("Some tests have failed"))
    } else {
        Ok(())
    }
}

/// Prepare and run the integration test
async fn run_integration_test(
    test_directory: PathBuf,
    postgres_docker: Arc<DockerTestClient>,
    postgres_ports: Arc<MappedPorts>,
    ipfs_ports: Arc<MappedPorts>,
    indexer_manager_bin: Arc<PathBuf>,
) -> anyhow::Result<IntegrationTestResult> {
    // build URIs
    let postgres_unique_id = get_unique_postgres_counter();
    let postgres_uri = make_postgres_uri(&postgres_unique_id, &postgres_ports);
    let ipfs_uri = make_ipfs_uri(&ipfs_ports);

    // create test database
    DockerTestClient::create_postgres_database(&postgres_docker, &postgres_unique_id)
        .await
        .context("failed to create the test database.")?;

    // prepare to run test command
    let test_setup = IntegrationTestSetup {
        postgres_uri,
        ipfs_uri,
        indexer_manager_bin,
        indexer_manager_ports: IndexerManagerPorts::get_ports(),
        test_directory,
    };

    // spawn indexer-manager
    let mut indexer_manager_child_command = run_indexer_manager(&test_setup).await?;

    println!("Test started: {}", basename(&test_setup.test_directory));

    let indexer_manger_stdio = stop_indexer_manager(&mut indexer_manager_child_command).await?;

    Ok(IntegrationTestResult {
        test_setup,
        test_command_results,
        indexer_manager_stdio: indexer_manger_stdio,
    })
}

async fn run_indexer_manager(test_setup: &IntegrationTestSetup) -> anyhow::Result<Child> {
    use std::process::Stdio;

    let mut command = Command::new(test_setup.indexer_manager_bin.as_os_str());
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    command
        .spawn()
        .context("failed to start indexer-manager command")
}

async fn stop_indexer_manager(child: &mut Child) -> anyhow::Result<StdIO> {
    child
        .kill()
        .await
        .context("Failed to kill indexer-manager")?;

    // capture stdio
    let stdout = match child.stdout.take() {
        Some(mut data) => Some(process_stdio(&mut data, "[indexer-manager:stdout] ").await?),
        None => None,
    };
    let stderr = match child.stderr.take() {
        Some(mut data) => Some(process_stdio(&mut data, "[indexer-manager:stderr] ").await?),
        None => None,
    };

    Ok(StdIO { stdout, stderr })
}

async fn process_stdio<T: AsyncReadExt + Unpin>(
    stdio: &mut T,
    prefix: &str,
) -> anyhow::Result<String> {
    let mut buffer: Vec<u8> = Vec::new();
    stdio
        .read_to_end(&mut buffer)
        .await
        .context("failed to read stdio")?;
    Ok(pretty_output(&buffer, prefix))
}

/// run yarn to build everything
async fn run_yarn_command(base_directory: &impl AsRef<Path>) {
    println!("Running `yarn` command in integration tests root directory.");
    let output = Command::new("yarn")
        .current_dir(base_directory)
        .output()
        .await
        .expect("failed to run yarn command");

    if output.status.success() {
        return;
    }
    println!("Yarn command failed.");
    println!("{}", pretty_output(&output.stdout, "[yarn:stdout]"));
    println!("{}", pretty_output(&output.stderr, "[yarn:stderr]"));
    panic!("Yarn command failed.")
}

fn parse_numeric_environment_variable(environment_variable_name: &str) -> Option<u64> {
    std::env::var(environment_variable_name)
        .ok()
        .and_then(|x| x.parse().ok())
}
