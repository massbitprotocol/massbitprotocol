// use crate::prelude::CheapClone;
use anyhow::Error;
use bytes::Bytes;
use futures03::{Stream, TryFutureExt};
use http::header::CONTENT_LENGTH;
use http::Uri;
use reqwest::multipart;
use serde::Deserialize;
use std::{str::FromStr, sync::Arc};
// use tokio_compat_02::FutureExt;
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectStatResponse {
    pub hash: String,
    pub num_links: u64,
    pub block_size: u64,
    pub links_size: u64,
    pub data_size: u64,
    pub cumulative_size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AddResponse {
    pub name: String,
    pub hash: String,
    pub size: String,
}

#[derive(Clone)]
pub struct IpfsClient {
    base: Arc<Uri>,
    client: Arc<reqwest::Client>,
}

// impl CheapClone for IpfsClient {
//     fn cheap_clone(&self) -> Self {
//         IpfsClient {
//             base: self.base.cheap_clone(),
//             client: self.client.cheap_clone(),
//         }
//     }
// }

impl IpfsClient {
    pub fn new(base: &str) -> Result<Self, Error> {
        Ok(IpfsClient {
            client: Arc::new(reqwest::Client::new()),
            base: Arc::new(Uri::from_str(base)?),
        })
    }

    pub fn localhost() -> Self {
        IpfsClient {
            client: Arc::new(reqwest::Client::new()),
            base: Arc::new(Uri::from_str("http://localhost:5001").unwrap()),
        }
    }

    /// Calls `object stat`.
    pub async fn object_stat(&self, path: String) -> Result<ObjectStatResponse, reqwest::Error> {
        self.call(self.url("object/stat", path), None)
            .await?
            .json()
            .await
    }

    /// Download the entire contents.
    pub async fn cat_all(&self, cid: String) -> Result<Bytes, reqwest::Error> {
        self.call(self.url("cat", cid), None).await?.bytes().await
    }

    pub async fn cat(
        &self,
        cid: String,
    ) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, reqwest::Error> {
        Ok(self.call(self.url("cat", cid), None).await?.bytes_stream())
    }

    pub async fn test(&self) -> Result<(), reqwest::Error> {
        self.call(format!("{}api/v0/version", self.base), None)
            .await
            .map(|_| ())
    }

    pub async fn add(&self, data: Vec<u8>) -> Result<AddResponse, reqwest::Error> {
        let form = multipart::Form::new().part("path", multipart::Part::bytes(data));

        self.call(format!("{}api/v0/add", self.base), Some(form))
            .await?
            .json()
            .await
    }

    fn url(&self, route: &'static str, arg: String) -> String {
        // URL security: We control the base and the route, user-supplied input goes only into the
        // query parameters.
        format!("{}api/v0/{}?arg={}", self.base, route, arg)
    }

    async fn call(
        &self,
        url: String,
        form: Option<multipart::Form>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let mut req = self.client.post(&url);
        if let Some(form) = form {
            req = req.multipart(form);
        } else {
            // Some servers require `content-length` even for an empty body.
            req = req.header(CONTENT_LENGTH, 0);
        }
        req.send()
            .await
            .map(|res| res.error_for_status())
            .and_then(|x| x)
    }
}


// fn create_ipfs_clients(logger: &Logger, ipfs_addresses: &Vec<String>) -> Vec<IpfsClient> {
async fn create_ipfs_clients(ipfs_addresses: &Vec<String>) -> Vec<IpfsClient> {
    // Parse the IPFS URL from the `--ipfs` command line argument
    let ipfs_addresses: Vec<_> = ipfs_addresses
        .iter()
        .map(|uri| {
            if uri.starts_with("http://") || uri.starts_with("https://") {
                String::from(uri)
            } else {
                format!("http://{}", uri)
            }
        })
        .collect();

    ipfs_addresses
        .into_iter()
        .map(|ipfs_address| {
            // info!(
            //     logger,
            //     "Trying IPFS node at: {}",
            //     SafeDisplay(&ipfs_address)
            // );
            println!("Trying IPFS node at");

            let ipfs_client = match IpfsClient::new(&ipfs_address) {
                Ok(ipfs_client) => ipfs_client,
                Err(e) => {
                    // error!(
                    //     logger,
                    //     "Failed to create IPFS client for `{}`: {}",
                    //     SafeDisplay(&ipfs_address),
                    //     e
                    // );
                    println!("Failed to create IPFS client");
                    panic!("Could not connect to IPFS");
                }
            };
            // Test the IPFS client by getting the version from the IPFS daemon
            // let ipfs_test = ipfs_client.cheap_clone();
            // let ipfs_ok_logger = logger.clone();
            // let ipfs_err_logger = logger.clone();
            let ipfs_address_for_ok = ipfs_address.clone();
            let ipfs_address_for_err = ipfs_address.clone();

            let ipfs_test = ipfs_client.clone();
            tokio::spawn(async move {
                ipfs_test
                    .test()
                    .map_err(move |e| {
                        // error!(
                        //     ipfs_err_logger,
                        //     "Is there an IPFS node running at \"{}\"?",
                        //     SafeDisplay(ipfs_address_for_err),
                        // );
                        panic!("Failed to connect to IPFS: {}", e);
                    })
                    .map_ok(move |_| {
                        println!("Successfully connected to IPFS node")
                        // info!(
                        //     ipfs_ok_logger,
                        //     "Successfully connected to IPFS node at: {}",
                        //     SafeDisplay(ipfs_address_for_ok)
                        // );
                    }).await;
            });
            thread::sleep(Duration::from_secs(3)); // Todo: check why ipfs is not waiting
            ipfs_client
        })
        .collect()
}

#[tokio::main]
async fn main() {
    let mut ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
    create_ipfs_clients(&ipfs_addresses).await;
}
