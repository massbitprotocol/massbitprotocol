#[derive(Debug, Clone, Default)]
pub struct AccessControl {
    pub access_control_allow_headers: String,
    pub access_control_allow_origin: String,
    pub access_control_allow_methods: String,
    pub content_type: String,
}
impl AccessControl {
    pub fn get_access_control_allow_headers(&self) -> Vec<String> {
        self.access_control_allow_headers
            .split(",")
            .into_iter()
            .map(|header| header.replace(" ", ""))
            .collect()
    }
}
