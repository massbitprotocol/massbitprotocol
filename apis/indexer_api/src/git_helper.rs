use massbit_common::prelude::anyhow;
use octocrab::Octocrab;
use std::io::Cursor;

pub struct GitHelper {
    pub repo: String,
}
impl GitHelper {
    pub fn new(repo: &String) -> Self {
        GitHelper { repo: repo.clone() }
    }
    pub async fn load_indexer(&self) -> Result<(), anyhow::Error> {
        let octocrab = Octocrab::builder().build()?;
        let builder = octocrab.request_builder(&self.repo, reqwest::Method::GET);
        let response = octocrab.execute(builder).await?;
        println!("{:?}", &response);
        let content = octocrab
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
            let builder = octocrab.request_builder(&first_item.url, reqwest::Method::GET);
            let response = octocrab.execute(builder).await?;
            println!("{:?}", &response);
            println!("{:?}", first_item);
        }
        //
        // println!(
        //     "{} files/dirs in the repo root",
        //     content.items.into_iter().count()
        // );

        // let response = reqwest::get(&self.repo).await?;
        // let mut file = std::fs::File::create(file_name)?;
        // let mut content = Cursor::new(response.bytes().await?);
        // std::io::copy(&mut content, &mut file)?;
        Ok(())
    }
}
