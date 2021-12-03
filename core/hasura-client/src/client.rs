use anyhow::anyhow;
use http::{Method, Uri};
use massbit_common::prelude::anyhow::Error;
use massbit_common::prelude::serde_json::{json, Value};
use massbit_common::prelude::tokio_compat_02::FutureExt;
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct HasuraClient {
    base: Arc<Uri>,
    client: Arc<Client>,
}
impl HasuraClient {
    pub fn new(base: &str) -> Result<Self, Error> {
        Ok(HasuraClient {
            client: Arc::new(Client::new()),
            base: Arc::new(Uri::from_str(base)?),
        })
    }
    fn build_request(&self, method: Method, endpoint: &str) -> Result<RequestBuilder, Error> {
        let mut builder = Uri::builder();
        if let Some(schema) = self.base.scheme_str() {
            builder = builder.scheme(schema);
        }
        if let Some(authority) = self.base.authority() {
            builder = builder.authority(authority.clone());
        }
        if endpoint.starts_with('/') {
            builder = builder.path_and_query(endpoint);
        } else {
            builder = builder.path_and_query(format!("/{}", endpoint));
        }

        let url = builder.build()?.to_string();
        Ok(self.client.request(method, &url))
    }
    pub async fn call_hasura_api<T: Serialize + ?Sized, V: DeserializeOwned>(
        &self,
        method: Method,
        endpoint: &str,
        payload: Option<&T>,
    ) -> Result<V, Error> {
        let mut req = self.build_request(method, endpoint)?;
        if let Some(payload) = payload {
            req = req.json(payload);
        }
        match req.send().compat().await {
            Ok(res) => res.json::<V>().await.map_err(|err| anyhow!("{:?}", &err)),
            Err(err) => Err(anyhow!("{:?}", &err)),
        }
    }
}

impl HasuraClient {
    pub async fn get_metadata(&self, schema_name: &str) -> Result<Value, anyhow::Error> {
        let payload = json!({
            "type" : "export_metadata",
            "version" : 2,
            "args" : {}
        });
        match self
            .call_hasura_api::<Value, crate::models::MetadataResource>(
                Method::POST,
                "/v1/metadata",
                Some(&payload),
            )
            .await
            .as_mut()
        {
            Ok(res) => {
                res.metadata.filter(schema_name);
                Ok(json!(res))
            }
            Err(err) => Err(anyhow!("{:?}", &err)),
        }
    }
    pub async fn get_graphql_schema(
        &self,
        query: &String,
        schema_name: &str,
    ) -> Result<Value, anyhow::Error> {
        let payload = json!({ "query": query });
        match self
            .call_hasura_api::<Value, crate::models::GraphqlSchemaResponse>(
                Method::POST,
                "/v1/graphql",
                Some(&payload),
            )
            .await
            .as_mut()
        {
            Ok(res) => {
                res.data.schema.filter(schema_name);
                Ok(json!(res))
            }
            Err(err) => Err(anyhow!("{:?}", &err)),
        }
    }
}
