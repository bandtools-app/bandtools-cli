use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use reqwest::{
    Method, StatusCode,
    blocking::{Client as HttpClient, multipart},
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, LOCATION, USER_AGENT},
};
use serde_json::Value;

use crate::config::ResolvedConfig;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QueryParams(Vec<(String, String)>);

impl QueryParams {
    pub fn push_opt<T: ToString>(&mut self, key: &str, value: Option<T>) {
        if let Some(value) = value {
            self.0.push((key.to_string(), value.to_string()));
        }
    }

    pub fn push(&mut self, key: &str, value: impl ToString) {
        self.0.push((key.to_string(), value.to_string()));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_slice(&self) -> &[(String, String)] {
        &self.0
    }
}

#[derive(Clone)]
pub struct ApiClient {
    http: HttpClient,
    base_url: String,
    token: String,
}

#[derive(Clone, Copy, Debug)]
pub enum Body<'a> {
    Empty,
    Json(&'a Value),
}

#[derive(Clone, Debug)]
pub struct ApiResponse {
    pub status: StatusCode,
    pub body: Option<Value>,
    pub location: Option<String>,
}

impl ApiClient {
    pub fn new(config: &ResolvedConfig) -> Result<Self> {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!("bt/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            http,
            base_url: config.api_url.clone(),
            token: config.api_token.clone(),
        })
    }

    pub fn request(
        &self,
        method: Method,
        path: &str,
        query: &QueryParams,
        body: Body<'_>,
    ) -> Result<ApiResponse> {
        let url = self.url(path, query)?;
        let mut request = self
            .http
            .request(method, url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, format!("bt/{}", env!("CARGO_PKG_VERSION")));

        if let Body::Json(value) = body {
            request = request.json(value);
        }

        let response = request.send().context("request failed")?;
        Self::decode(response)
    }

    pub fn multipart_file(&self, path: &str, field: &str, file: PathBuf) -> Result<ApiResponse> {
        let url = self.url(path, &QueryParams::default())?;
        let filename = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("upload")
            .to_string();
        let part = multipart::Part::file(&file)
            .with_context(|| format!("failed to read upload file {}", file.display()))?
            .file_name(filename);
        let form = multipart::Form::new().part(field.to_string(), part);

        let response = self
            .http
            .request(Method::PUT, url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/json")
            .multipart(form)
            .send()
            .context("request failed")?;
        Self::decode(response)
    }

    pub fn multipart_post_file(
        &self,
        path: &str,
        field: &str,
        file: PathBuf,
    ) -> Result<ApiResponse> {
        let url = self.url(path, &QueryParams::default())?;
        let filename = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("upload")
            .to_string();
        let part = multipart::Part::file(&file)
            .with_context(|| format!("failed to read upload file {}", file.display()))?
            .file_name(filename);
        let form = multipart::Form::new().part(field.to_string(), part);

        let response = self
            .http
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/json")
            .multipart(form)
            .send()
            .context("request failed")?;
        Self::decode(response)
    }

    fn url(&self, path: &str, query: &QueryParams) -> Result<reqwest::Url> {
        let base = format!("{}/", self.base_url.trim_end_matches('/'));
        let mut url = reqwest::Url::parse(&base)
            .and_then(|base| base.join(path.trim_start_matches('/')))
            .with_context(|| format!("failed to build request URL for path {path}"))?;

        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query.as_slice());
        }

        Ok(url)
    }

    fn decode(response: reqwest::blocking::Response) -> Result<ApiResponse> {
        let status = response.status();
        let location = header_to_string(response.headers(), LOCATION);
        let content_type = header_to_string(response.headers(), CONTENT_TYPE);
        let bytes = response.bytes().context("failed to read response body")?;
        let body = if bytes.is_empty() {
            None
        } else if content_type
            .as_deref()
            .is_some_and(|value| value.contains("application/json"))
        {
            Some(serde_json::from_slice(&bytes).context("failed to parse JSON response")?)
        } else {
            let text = String::from_utf8_lossy(&bytes).to_string();
            Some(serde_json::json!({ "data": { "body": text } }))
        };

        if status.is_success() || status.is_redirection() {
            return Ok(ApiResponse {
                status,
                body,
                location,
            });
        }

        let message = body
            .as_ref()
            .and_then(|value| value.pointer("/error/message"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                body.as_ref()
                    .and_then(|value| value.pointer("/error/code"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| status.to_string());

        Err(anyhow!("BandTools API returned {status}: {message}"))
    }
}

pub fn json_from_data(data: Option<String>, data_file: Option<PathBuf>) -> Result<Value> {
    match (data, data_file) {
        (Some(_), Some(_)) => bail!("use either --data or --data-file, not both"),
        (Some(raw), None) => serde_json::from_str(&raw).context("failed to parse --data as JSON"),
        (None, Some(path)) => {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read JSON file {}", path.display()))?;
            serde_json::from_str(&raw)
                .with_context(|| format!("failed to parse JSON file {}", path.display()))
        }
        (None, None) => bail!("missing request body; pass --data or --data-file"),
    }
}

fn header_to_string(headers: &HeaderMap, name: reqwest::header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value: &HeaderValue| value.to_str().ok())
        .map(str::to_string)
}

pub fn response_json(response: ApiResponse) -> Value {
    let mut body = response.body.unwrap_or_else(|| serde_json::json!({}));
    if let Some(location) = response.location
        && let Some(object) = body.as_object_mut()
    {
        object.insert("location".to_string(), Value::String(location));
    }
    body
}

pub fn ensure_no_body(response: ApiResponse) -> Value {
    if let Some(body) = response.body {
        body
    } else {
        serde_json::json!({
            "data": {
                "status": response.status.as_u16()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_json() {
        let value = json_from_data(Some(r#"{"name":"Tour"}"#.to_string()), None).unwrap();
        assert_eq!(value["name"], "Tour");
    }

    #[test]
    fn rejects_two_json_sources() {
        let error = json_from_data(Some("{}".to_string()), Some(PathBuf::from("body.json")))
            .unwrap_err()
            .to_string();
        assert!(error.contains("either --data or --data-file"));
    }
}
