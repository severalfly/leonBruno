use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod { Get, Post, Put, Patch, Delete, Head, Options }

impl Default for HttpMethod {
    fn default() -> Self { HttpMethod::Get }
}
impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET", HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT", HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE", HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "POST" => HttpMethod::Post, "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch, "DELETE" => HttpMethod::Delete,
            "HEAD" => HttpMethod::Head, "OPTIONS" => HttpMethod::Options,
            _ => HttpMethod::Get,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Auth {
    pub kind: String,   // none | bearer | basic
    #[serde(default)] pub token: String,
    #[serde(default)] pub username: String,
    #[serde(default)] pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)] pub headers: Vec<KeyValue>,
    #[serde(default)] pub params: Vec<KeyValue>,
    #[serde(default)] pub body: String,
    #[serde(default)] pub body_type: String, // none | raw | json | form
    #[serde(default)] pub auth: Auth,
}

impl Default for ApiRequest {
    fn default() -> Self {
        Self {
            id: String::new(), name: "New Request".into(),
            method: HttpMethod::Get, url: String::new(),
            headers: Vec::new(), params: Vec::new(), body: String::new(),
            body_type: "none".into(), auth: Auth { kind: "none".into(), ..Default::default() },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValue>,
    pub body: String,
    pub elapsed_ms: u128,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub collections: Vec<Collection>,
    #[serde(default)] pub environments: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(default)] pub requests: Vec<ApiRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    #[serde(default)] pub workspace_dir: String,
    #[serde(default)] pub active_environment: String,
    #[serde(default = "default_timeout")] pub timeout_secs: u64,
}
fn default_timeout() -> u64 { 30 }
