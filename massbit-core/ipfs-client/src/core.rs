use anyhow::Error;
use bytes::Bytes;
use futures03::{Stream, TryFutureExt};
use http::header::CONTENT_LENGTH;
use http::Uri;
use reqwest::multipart;
use serde::Deserialize;
use std::{str::FromStr, sync::Arc, thread};
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

pub async fn create_ipfs_clients(ipfs_addresses: &Vec<String>) -> Vec<IpfsClient> {
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
            log::info!("[Ipfs Client] Try connecting to IPFS node");
            let ipfs_client = match IpfsClient::new(&ipfs_address) {
                Ok(ipfs_client) => ipfs_client,
                Err(e) => {
                    log::error!("[Ipfs Client] Failed to create IPFS client {}", e);
                    panic!("Could not connect to IPFS");
                }
            };

            let ipfs_test = ipfs_client.clone();
            // Hughie: comment out the check for connection because there's an error with tokio spawm runtime
            // We can use tokio02 spawn custom function to fix this problem
            // #[allow(unused_must_use)]
            // tokio::spawn(async move {
            //     ipfs_test
            //         .test()
            //         .map_err(move |e| {
            //             panic!("[Ipfs Client] Failed to connect to IPFS: {}", e);
            //         })
            //         .map_ok(move |_| {
            //             log::info!("[Ipfs Client] Successfully connected to IPFS node");
            //         }).await;
            // });
            ipfs_client
        })
        .collect()
}
