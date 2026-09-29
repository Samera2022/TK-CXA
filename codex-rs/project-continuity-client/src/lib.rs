use serde::{Deserialize, Serialize};
use url::Url;

const CONTINUITY_PATH: &str = "/__ccr/continuity/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectCheckpoint {
    pub id: String,
    #[serde(rename = "projectDirectory")]
    pub project_directory: String,
    pub client: String,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    #[serde(rename = "gitHeadSha")]
    pub git_head_sha: Option<String>,
    #[serde(rename = "gitDirty")]
    pub git_dirty: bool,
    pub objective: Option<String>,
    pub completed: Vec<String>,
    pub blockers: Vec<String>,
    #[serde(rename = "nextSteps")]
    pub next_steps: Vec<String>,
    #[serde(rename = "modifiedFiles")]
    pub modified_files: Vec<String>,
    #[serde(rename = "verificationStatus")]
    pub verification_status: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitState {
    pub branch: Option<String>,
    #[serde(rename = "headSha")]
    pub head_sha: Option<String>,
    pub dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateCheckpointRequest {
    pub id: Option<String>,
    #[serde(rename = "projectDirectory")]
    pub project_directory: String,
    pub client: String,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    #[serde(rename = "gitHeadSha")]
    pub git_head_sha: Option<String>,
    #[serde(rename = "gitDirty")]
    pub git_dirty: bool,
    pub objective: Option<String>,
    pub completed: Vec<String>,
    pub blockers: Vec<String>,
    #[serde(rename = "nextSteps")]
    pub next_steps: Vec<String>,
    #[serde(rename = "modifiedFiles")]
    pub modified_files: Vec<String>,
    #[serde(rename = "verificationStatus")]
    pub verification_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct CheckpointResponse {
    checkpoint: ProjectCheckpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct LatestCheckpointResponse {
    checkpoint: Option<ProjectCheckpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct GitStateResponse {
    #[serde(rename = "gitState")]
    git_state: GitState,
}

#[derive(Debug, thiserror::Error)]
pub enum ContinuityClientError {
    #[error("invalid CCR base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("CCR continuity request failed with status {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("CCR continuity transport failed: {0}")]
    Transport(#[from] reqwest::Error),
}

#[derive(Clone, Debug)]
pub struct ContinuityClient {
    client: reqwest::Client,
    base_url: Url,
    token: Option<String>,
}

impl ContinuityClient {
    pub fn new(base_url: &str, token: Option<String>) -> Result<Self, ContinuityClientError> {
        let base_url = normalize_base_url(Url::parse(base_url.trim())?);
        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
            token: token.filter(|value| !value.trim().is_empty()),
        })
    }

    pub fn endpoint_url(&self, suffix: &str) -> Url {
        let mut url = self.base_url.clone();
        url.set_path(&format!("{CONTINUITY_PATH}{suffix}"));
        url
    }

    pub async fn create_checkpoint(
        &self,
        checkpoint: &CreateCheckpointRequest,
    ) -> Result<ProjectCheckpoint, ContinuityClientError> {
        let response = self
            .authorized(
                self.client
                    .post(self.endpoint_url("/checkpoints"))
                    .json(checkpoint),
            )
            .send()
            .await?;
        parse_response(response)
            .await
            .map(|body: CheckpointResponse| body.checkpoint)
    }

    pub async fn get_latest(
        &self,
        project_directory: &str,
        git_branch: Option<&str>,
    ) -> Result<Option<ProjectCheckpoint>, ContinuityClientError> {
        let mut url = self.endpoint_url("/checkpoints/latest");
        url.query_pairs_mut()
            .append_pair("projectDirectory", project_directory);
        if let Some(branch) = git_branch {
            url.query_pairs_mut().append_pair("gitBranch", branch);
        }
        let response = self.authorized(self.client.get(url)).send().await?;
        parse_response(response)
            .await
            .map(|body: LatestCheckpointResponse| body.checkpoint)
    }

    pub async fn get_git_state(
        &self,
        project_directory: &str,
    ) -> Result<GitState, ContinuityClientError> {
        let mut url = self.endpoint_url("/git-state");
        url.query_pairs_mut()
            .append_pair("projectDirectory", project_directory);
        let response = self.authorized(self.client.get(url)).send().await?;
        parse_response(response)
            .await
            .map(|body: GitStateResponse| body.git_state)
    }

    fn authorized(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(token) = self.token.as_deref() {
            request.bearer_auth(token)
        } else {
            request
        }
    }
}

async fn parse_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, ContinuityClientError> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ContinuityClientError::HttpStatus {
            status: status.as_u16(),
            body,
        });
    }
    Ok(response.json().await?)
}

fn normalize_base_url(mut url: Url) -> Url {
    let path = url.path().trim_end_matches('/').to_owned();
    let path = path.strip_suffix("/v1").unwrap_or(&path);
    url.set_path(path.trim_end_matches('/'));
    url
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn request() -> CreateCheckpointRequest {
        CreateCheckpointRequest {
            id: Some("chk_test".to_owned()),
            project_directory: "/tmp/project".to_owned(),
            client: "codex".to_owned(),
            git_branch: Some("main".to_owned()),
            git_head_sha: Some("abc123".to_owned()),
            git_dirty: false,
            objective: Some("Continue".to_owned()),
            completed: vec!["Scaffold".to_owned()],
            blockers: vec![],
            next_steps: vec!["Integrate".to_owned()],
            modified_files: vec!["src/lib.rs".to_owned()],
            verification_status: Some("passed".to_owned()),
        }
    }

    fn checkpoint() -> ProjectCheckpoint {
        ProjectCheckpoint {
            id: "chk_test".to_owned(),
            project_directory: "/tmp/project".to_owned(),
            client: "codex".to_owned(),
            git_branch: Some("main".to_owned()),
            git_head_sha: Some("abc123".to_owned()),
            git_dirty: false,
            objective: Some("Continue".to_owned()),
            completed: vec!["Scaffold".to_owned()],
            blockers: vec![],
            next_steps: vec!["Integrate".to_owned()],
            modified_files: vec!["src/lib.rs".to_owned()],
            verification_status: Some("passed".to_owned()),
            created_at: 1,
            updated_at: 2,
        }
    }

    #[test]
    fn normalizes_ccr_provider_urls() {
        let client = ContinuityClient::new("http://127.0.0.1:3456/v1/", None).unwrap();
        assert_eq!(
            client.endpoint_url("/checkpoints").as_str(),
            "http://127.0.0.1:3456/__ccr/continuity/v1/checkpoints"
        );
    }

    #[tokio::test]
    async fn creates_checkpoint_with_bearer_auth() {
        let server = MockServer::start().await;
        let expected = checkpoint();
        Mock::given(method("POST"))
            .and(path("/__ccr/continuity/v1/checkpoints"))
            .and(header("authorization", "Bearer secret"))
            .and(body_json(serde_json::to_value(request()).unwrap()))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "checkpoint": expected
            })))
            .mount(&server)
            .await;

        let client = ContinuityClient::new(&server.uri(), Some("secret".to_owned())).unwrap();
        assert_eq!(
            client.create_checkpoint(&request()).await.unwrap(),
            expected
        );
    }

    #[tokio::test]
    async fn gets_latest_and_git_state() {
        let server = MockServer::start().await;
        let expected = checkpoint();
        Mock::given(method("GET"))
            .and(path("/__ccr/continuity/v1/checkpoints/latest"))
            .and(query_param("projectDirectory", "/tmp/project"))
            .and(query_param("gitBranch", "main"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "checkpoint": expected
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/__ccr/continuity/v1/git-state"))
            .and(query_param("projectDirectory", "/tmp/project"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "gitState": { "branch": "main", "headSha": "abc123", "dirty": false }
            })))
            .mount(&server)
            .await;

        let client = ContinuityClient::new(&server.uri(), None).unwrap();
        assert_eq!(
            client
                .get_latest("/tmp/project", Some("main"))
                .await
                .unwrap(),
            Some(expected)
        );
        assert_eq!(
            client.get_git_state("/tmp/project").await.unwrap(),
            GitState {
                branch: Some("main".to_owned()),
                head_sha: Some("abc123".to_owned()),
                dirty: false
            }
        );
    }

    #[tokio::test]
    async fn returns_http_status_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/__ccr/continuity/v1/git-state"))
            .respond_with(ResponseTemplate::new(503).set_body_string("offline"))
            .mount(&server)
            .await;
        let client = ContinuityClient::new(&server.uri(), None).unwrap();
        assert!(matches!(
            client.get_git_state("/tmp/project").await,
            Err(ContinuityClientError::HttpStatus { status: 503, body }) if body == "offline"
        ));
    }
}
