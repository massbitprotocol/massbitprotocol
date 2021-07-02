use ipfs_client::core::create_ipfs_clients;
use tokio_compat_02::FutureExt;

// Should be made into a struct, add just add the logic to ipfs_client struct
pub async fn get_config(ipfs_config_hash: &String) -> serde_yaml::Mapping {
    // Shouldn't be re-created
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

    // yaml QmPxYkWaNa2wov6pRvRY7pL8Fk4a6zDtV9hbJKpmj61EEq
    // so QmSQwVnx167vdvkvzstUmDXUC2SkB56KYP7W7cawKZ6Utf
    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_config_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let raw: serde_yaml::Mapping = serde_yaml::from_slice(&file_bytes).unwrap();
    raw
}

pub async fn get_mapping_file(ipfs_mapping_hash: &String) -> serde_yaml::Mapping {
    // Shouldn't be re-created
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

    // yaml QmPxYkWaNa2wov6pRvRY7pL8Fk4a6zDtV9hbJKpmj61EEq
    // so QmSQwVnx167vdvkvzstUmDXUC2SkB56KYP7W7cawKZ6Utf
    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_mapping_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let raw: serde_yaml::Mapping = serde_yaml::from_slice(&file_bytes).unwrap();
    raw
}