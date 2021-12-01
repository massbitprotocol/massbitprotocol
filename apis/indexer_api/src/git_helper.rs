use crate::FILES;
use anyhow::anyhow;
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
    fn get_owner_repo(&self) -> Result<(String, String), anyhow::Error> {
        // git@github.com:massbitprotocol/serum_index.git
        // https://github.com/massbitprotocol/serum_index
        // https://github.com/massbitprotocol/serum_index.git
        let url = self.repo.clone();
        let res: Vec<_> = url
            .trim_end_matches(".git")
            .trim_start_matches("git@github.com:")
            .trim_start_matches("https://github.com/")
            .split("/")
            .collect();
        if res.len() == 2 {
            return Ok((res[0].to_string(), res[1].to_string()));
        }
        return Err(anyhow!("Invalid repo url: {}", &self.repo));
    }
    pub async fn load_indexer(&self) -> Result<HashMap<String, Bytes>, anyhow::Error> {
        log::info!("Repo url: {:#?}", &self.repo);
        let (owner, repo_name) = self.get_owner_repo()?;
        log::info!("owner: {}, repo_name: {}", &owner, &repo_name);

        let mut map = HashMap::default();
        let content = self
            .github_client
            .repos(owner, repo_name)
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
                                map.insert(FILES.get(&content.name).unwrap().clone(), bytes);
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
        if let Some(url) = &content.download_url {
            println!("download_file url:{}", &url);
            if let Ok(response) = self.get_github_content(url).await {
                resp = response.bytes().await.ok();
            }
        }
        resp
    }
}
