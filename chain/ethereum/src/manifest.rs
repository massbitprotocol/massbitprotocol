use std::collections::HashMap;
use std::time::Duration;

use massbit::components::link_resolver::{JsonValueStream, LinkResolver as LinkResolverTrait};
use massbit::data::indexer::{IndexerManifest, Link};
use massbit::prelude::{anyhow, async_trait, serde_yaml, DeploymentHash, Error};

use crate::Chain;

#[derive(Default)]
pub struct TextResolver {
    texts: HashMap<String, Vec<u8>>,
}

impl TextResolver {
    fn add(&mut self, link: &str, text: &impl AsRef<[u8]>) {
        self.texts.insert(
            link.to_owned(),
            text.as_ref().into_iter().cloned().collect(),
        );
    }
}

#[async_trait]
impl LinkResolverTrait for TextResolver {
    fn with_timeout(self, _timeout: Duration) -> Self {
        self
    }

    fn with_retries(self) -> Self {
        self
    }

    async fn cat(&self, link: &Link) -> Result<Vec<u8>, Error> {
        self.texts
            .get(&link.link)
            .ok_or(anyhow!("No text for {}", &link.link))
            .map(Clone::clone)
    }

    async fn json_stream(&self, _link: &Link) -> Result<JsonValueStream, Error> {
        unimplemented!()
    }
}

pub async fn resolve_manifest_from_text(text: &str) -> IndexerManifest<Chain> {
    let mut resolver = TextResolver::default();
    resolver.add("Qmmanifest", &text);
    let raw = serde_yaml::from_str(text).unwrap();
    let deployment_hash = DeploymentHash::new("Qmmanifest".to_string()).unwrap();

    IndexerManifest::resolve_from_raw(deployment_hash, raw, &resolver)
        .await
        .expect("Parsing simple manifest works")
}
