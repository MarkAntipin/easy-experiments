use serde::Deserialize;

#[derive(Deserialize)]
pub struct GetExperimentsQueryParams {
    #[serde(default)]
    pub status: Option<String>,
}
