use std::collections::HashMap;
use std::time::Duration;

use massbit::components::link_resolver::{JsonValueStream, LinkResolver as LinkResolverTrait};
use massbit::data::indexer::{IndexerManifest, Link, SPEC_VERSION_0_0_4};
use massbit::prelude::{anyhow, async_trait, serde_yaml, DeploymentHash, Error};

use crate::Chain;
use massbit::log::logger;
use massbit::slog::Logger;

const GQL_SCHEMA: &str = "type Thing @entity { id: ID! }";
const GQL_SCHEMA_FULLTEXT: &str = include_str!("../../examples/full-text.graphql");
const MAPPING_WITH_IPFS_FUNC_WASM: &[u8] =
    include_bytes!("../../examples/ipfs-on-ethereum-contracts.wasm");
const ABI: &str = "[{\"type\":\"function\", \"inputs\": [{\"name\": \"i\",\"type\": \"uint256\"}],\"name\":\"get\",\"outputs\": [{\"type\": \"address\",\"name\": \"o\"}]}]";

#[derive(Default, Debug)]
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

    async fn cat(&self, logger: &Logger, link: &Link) -> Result<Vec<u8>, Error> {
        self.texts
            .get(&link.link)
            .ok_or(anyhow!("No text for {}", &link.link))
            .map(Clone::clone)
    }

    async fn json_stream(&self, logger: &Logger, _link: &Link) -> Result<JsonValueStream, Error> {
        unimplemented!()
    }
}

pub async fn resolve_manifest_from_text(text: &str) -> IndexerManifest<Chain> {
    let logger = logger(true);
    let mut resolver = TextResolver::default();
    let raw = serde_yaml::from_str(text).unwrap();
    let id = DeploymentHash::new("Qmmanifest").unwrap();
    resolver.add(id.as_str(), &text);
    resolver.add("/ipfs/Qmschema", &GQL_SCHEMA);
    resolver.add("/ipfs/Qmabi", &ABI);
    resolver.add("/ipfs/Qmmapping", &MAPPING_WITH_IPFS_FUNC_WASM);
    println!("text: {:#?}", &text);
    println!("resolver: {:#?}", &resolver);

    IndexerManifest::resolve_from_raw(&logger, id, raw, &resolver, SPEC_VERSION_0_0_4)
        .await
        .expect("Parsing simple manifest works")
}
