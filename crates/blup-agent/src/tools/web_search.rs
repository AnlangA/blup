use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::{SearchConfig, SearchProvider};

use super::{AgentTool, ToolError, ToolResult};

/// Web search tool supporting multiple providers.
pub struct WebSearchTool {
    http: Client,
    config: SearchConfig,
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Search response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub query: String,
}

impl WebSearchTool {
    pub fn new(config: SearchConfig) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self { http, config }
    }

    /// Execute a search query.
    pub async fn search(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<SearchResponse, ToolError> {
        match self.config.provider {
            SearchProvider::Brave => self.search_brave(query, num_results).await,
            SearchProvider::Exa => self.search_exa(query, num_results).await,
            SearchProvider::SearXNG => self.search_searxng(query, num_results).await,
            SearchProvider::None => Err(ToolError::ExecutionFailed(
                "No search provider configured. Set BLUP_SEARCH_PROVIDER and BLUP_SEARCH_API_KEY."
                    .to_string(),
            )),
        }
    }

    async fn search_brave(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<SearchResponse, ToolError> {
        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            ToolError::ExecutionFailed("Missing Brave Search API key".to_string())
        })?;

        let url = "https://api.search.brave.com/res/v1/web/search";
        let response = self
            .http
            .get(url)
            .header("Accept", "application/json")
            .header("X-Subscription-Token", api_key)
            .query(&[("q", query), ("count", &num_results.to_string())])
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed(format!(
                "Brave API error ({status}): {body}"
            )));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let results = data
            .get("web")
            .and_then(|w| w.get("results"))
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        Some(SearchResult {
                            title: r.get("title")?.as_str()?.to_string(),
                            url: r.get("url")?.as_str()?.to_string(),
                            snippet: r
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SearchResponse {
            results,
            query: query.to_string(),
        })
    }

    async fn search_exa(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<SearchResponse, ToolError> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("Missing Exa API key".to_string()))?;

        let url = "https://api.exa.ai/search";
        let body = json!({
            "query": query,
            "numResults": num_results,
            "type": "neural",
            "contents": {
                "text": true
            }
        });

        let response = self
            .http
            .post(url)
            .header("x-api-key", api_key)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed(format!(
                "Exa API error ({status}): {body_text}"
            )));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let results = data
            .get("results")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        Some(SearchResult {
                            title: r.get("title")?.as_str()?.to_string(),
                            url: r.get("url")?.as_str()?.to_string(),
                            snippet: r
                                .get("text")
                                .and_then(|t| t.as_str())
                                .unwrap_or("")
                                .chars()
                                .take(300)
                                .collect(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SearchResponse {
            results,
            query: query.to_string(),
        })
    }

    async fn search_searxng(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<SearchResponse, ToolError> {
        let base_url =
            self.config.base_url.as_ref().ok_or_else(|| {
                ToolError::ExecutionFailed("Missing SearXNG base URL".to_string())
            })?;

        let url = format!("{}/search", base_url.trim_end_matches('/'));
        let response = self
            .http
            .get(&url)
            .header("Accept", "application/json")
            .query(&[("q", query), ("format", "json"), ("pageno", "1")])
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed(format!(
                "SearXNG error ({status}): {body}"
            )));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let results = data
            .get("results")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .take(num_results)
                    .filter_map(|r| {
                        Some(SearchResult {
                            title: r.get("title")?.as_str()?.to_string(),
                            url: r.get("url")?.as_str()?.to_string(),
                            snippet: r
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SearchResponse {
            results,
            query: query.to_string(),
        })
    }
}

#[async_trait]
impl AgentTool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information. Returns titles, URLs, and snippets for relevant results."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return (default: 5)",
                    "default": 5,
                    "minimum": 1,
                    "maximum": 20
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let query = args
            .get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("Missing 'query' parameter".to_string()))?;

        let num_results = args
            .get("num_results")
            .and_then(|n| n.as_u64())
            .unwrap_or(5) as usize;

        let response = self.search(query, num_results).await?;

        let formatted = response
            .results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. **{}**\n   {}\n   {}", i + 1, r.title, r.snippet, r.url))
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(ToolResult::success(formatted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SearchProvider;

    fn create_search_tool(provider: SearchProvider) -> WebSearchTool {
        WebSearchTool::new(SearchConfig {
            provider,
            api_key: Some("test-key".to_string()),
            base_url: Some("http://localhost:8888".to_string()),
        })
    }

    #[test]
    fn test_tool_metadata() {
        let tool = create_search_tool(SearchProvider::Brave);
        assert_eq!(tool.name(), "web_search");
        assert!(!tool.description().is_empty());
        assert!(tool.parameters_schema().is_object());
    }

    #[test]
    fn test_parameters_schema_has_query() {
        let tool = create_search_tool(SearchProvider::Brave);
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("query")));
    }

    #[tokio::test]
    async fn test_no_provider_returns_error() {
        let tool = create_search_tool(SearchProvider::None);
        let result = tool.search("test", 5).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_query_returns_error() {
        let tool = create_search_tool(SearchProvider::Brave);
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            title: "Test".to_string(),
            url: "https://example.com".to_string(),
            snippet: "A test result".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("https://example.com"));
    }
}
