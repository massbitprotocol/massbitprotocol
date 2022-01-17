#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct IndexerConfig {
    pub smart_contract_source: String,
    pub main_instruction: String,
    pub unpack_function: String,
    pub start_block: u64,
    pub contract_address: String,
    pub output_unpacking: String,
    pub output_logic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
