use crate::FILES;
use bytes::Bytes;
use log::error;
use massbit_common::prelude::anyhow;
use octocrab::{models::repos::Content, Octocrab};
use std::collections::HashMap;

pub struct GitHelper {
    pub repo: String,
    github_client: Octocrab,
}
impl GitHelper {
    pub fn new(repo: &String) -> Self {
        let github_client = Octocrab::builder().build().unwrap();
        GitHelper {
            repo: repo.clone(),
            github_client,
        }
    }
    async fn get_github_content(&self, url: &String) -> octocrab::Result<reqwest::Response> {
        let builder = self
            .github_client
            .request_builder(url, reqwest::Method::GET);
        self.github_client.execute(builder).await
    }
    //Load indexer files (mapping, schema, graphql) from github
    //and store them in ipfs server
    pub async fn load_indexer(&self) -> Result<HashMap<String, Bytes>, anyhow::Error> {
        let mut map = HashMap::default();
        let content = self
            .github_client
            .repos("massbitprotocol", "serum_index")
            .get_content()
            .send()
            .await?;
        if let Some(first_item) = content
            .items
            .iter()
            .filter(|item| item.r#type.as_str() == "dir" && item.r#name.as_str() == "releases")
            .next()
        {
            let response = self.get_github_content(&first_item.url).await?;
            match response.json::<Vec<Content>>().await {
                Ok(contents) => {
                    for content in contents.iter() {
                        if FILES.contains_key(&content.name) {
                            if let Some(bytes) = self.download_file(content).await {
                                map.insert(content.name.clone(), bytes);
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("{:?}", &err);
                }
            }
        }
        Ok(map)
    }
    async fn download_file(&self, content: &Content) -> Option<Bytes> {
        let mut resp = None;
        if let Some(file_name) = FILES
            .iter()
            .filter(|(_, v)| content.name.eq(v.as_str()))
            .map(|(k, _)| k)
            .next()
        {
            if let Some(url) = &content.download_url {
                if let Ok(response) = self.get_github_content(url).await {
                    resp = response.bytes().await.ok();
                }
            }
        }
        resp
    }
}
