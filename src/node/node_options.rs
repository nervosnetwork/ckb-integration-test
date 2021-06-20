use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct NodeOptions {
    pub ckb_binary: PathBuf,
    pub initial_database: &'static str,
    pub chain_spec: &'static str,
    pub app_config: &'static str,
}
