use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use futures::Stream;
use serde_json::json;
use tokio::sync::Mutex;
use tracing;

use crate::audit::AuditLogger;
use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::mcp::McpManager;
use crate::memory::MemoryManager;
use crate::prompt::PromptLoader;
use crate::provider::{LlmMessage, LlmProvider, LlmRequest, ProviderFactory};
use crate::schema::SchemaValidator;
use crate::step::*;
use crate::tools::ToolRegistry;

pub struct AgentEngine {
    provider: Arc<dyn LlmProvider>,
    prompts: Arc<PromptLoader>,
    validator: Arc<SchemaValidator>,
    config: AgentConfig,
    memory: Mutex<MemoryManager>,
    audit: Option<Arc<AuditLogger>>,
    mcp: Mutex<McpManager>,
    tools: Arc<ToolRegistry>,
}

impl AgentEngine {
    /// Create an engine from configuration, building the LLM provider from
    /// the provider config.
    pub async fn new(config: AgentConfig) -> Result<Self, AgentError> {
        let provider = ProviderFactory::from_config(&config.provider)?;
        let prompts = Arc::new(PromptLoader::new(&config.prompts_dir));
        let validator = Arc::new(SchemaValidator::new(&config.schemas_dir));
        Ok(Self::with_provider(provider, prompts, validator, config).await)
    }

    /// Create an engine with a pre-built provider (useful for testing with
    /// mock providers).
    pub async fn with_provider(
        provider: Arc<dyn LlmProvider>,
        prompts: Arc<PromptLoader>,
        validator: Arc<SchemaValidator>,
        config: AgentConfig,
    ) -> Self {
        let audit = if config.audit.enabled {
            Some(Arc::new(AuditLogger::new(&config.audit)))
        } else {
            None
        };
        let memory = MemoryManager::new(&config.memory, Some(Arc::clone(&provider)));
        let mcp = McpManager::new(
            &config.mcp,
            config.memory.storage_dir.clone(),
            audit.clone(),
        )
        .await;

        let tools = Self::build_tools(&config);

        Self {
            provider,
            prompts,
            validator,
            config,
            memory: Mutex::new(memory),
            audit,
            mcp: Mutex::new(mcp),
            tools: Arc::new(tools),
        }
    }

    fn build_tools(config: &AgentConfig) -> ToolRegistry {
        let tools = ToolRegistry::new();
        tools.register(Arc::new(crate::tools::builtin::CalculatorTool));
        if config.search.provider != crate::config::SearchProvider::None {
            tools.register(Arc::new(crate::tools::web_search::WebSearchTool::new(
                config.search.clone(),
            )));
        }
        tools
    }

    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
    pub fn validator(&self) -> &SchemaValidator {
        &self.validator
    }
    pub fn provider(&self) -> &Arc<dyn LlmProvider> {
        &self.provider
    }
    pub fn tools(&self) -> &Arc<ToolRegistry> {
        &self.tools
    }
    pub fn audit(&self) -> Option<&Arc<AuditLogger>> {
        self.audit.as_ref()
    }
    pub fn memory(&self) -> &Mutex<MemoryManager> {
        &self.memory
    }
    pub fn mcp(&self) -> &Mutex<McpManager> {
        &self.mcp
    }

    pub async fn check_feasibility(
        &self,
        ctx: &FeasibilityContext,
    ) -> Result<serde_json::Value, AgentError> {
        let (system_prompt, user_prompt) = self.feasibility_prompts(ctx)?;
        self.llm_json(
            &system_prompt,
            &user_prompt,
            "feasibility_result",
            "feasibility",
        )
        .await
    }

    pub fn check_feasibility_stream(
        &self,
        ctx: FeasibilityContext,
    ) -> Pin<Box<dyn Stream<Item = Result<AgentStreamEvent, AgentError>> + Send>> {
        use futures::StreamExt;
        let prompts_result = self.feasibility_prompts(&ctx);
        let provider = Arc::clone(&self.provider);
        let validator = Arc::clone(&self.validator);
        let config = self.config.clone();
        let audit = self.audit.clone();

        Box::pin(async_stream::stream! {
            let (system_prompt, user_prompt) = match prompts_result { Ok(p) => p, Err(e) => { yield Err(e); return; } };
            yield Ok(AgentStreamEvent::Status { state: "FEASIBILITY_CHECK".to_string(), message: "Checking goal feasibility...".to_string() });
            let request = make_request(&config, &system_prompt, &user_prompt, true);
            let start = Instant::now();
            let chunk_stream = provider.stream(request);
            let mut full_text = String::new();
            let mut chunk_stream = std::pin::pin!(chunk_stream);
            while let Some(result) = chunk_stream.next().await {
                match result {
                    Ok(chunk) => { full_text.push_str(&chunk.content); yield Ok(AgentStreamEvent::Chunk { content: chunk.content, index: chunk.index }); }
                    Err(e) => { yield Ok(AgentStreamEvent::Error { code: "LLM_ERROR".to_string(), message: e.to_string() }); return; }
                }
            }
            let duration_ms = start.elapsed().as_millis() as u64;
            let json_str = extract_json(&full_text);
            match serde_json::from_str::<serde_json::Value>(&json_str) {
                Ok(parsed) => {
                    if let Err(e) = validator.validate(&parsed, "feasibility_result") {
                        if let Some(ref audit) = audit { audit.log_llm_call("stream", provider.name(), provider.model(), &Default::default(), duration_ms, false, Some(e.to_string())); }
                        yield Ok(AgentStreamEvent::Error { code: "VALIDATION_ERROR".to_string(), message: e.to_string() }); return;
                    }
                    if let Some(ref audit) = audit { audit.log_llm_call("stream", provider.name(), provider.model(), &Default::default(), duration_ms, true, None); }
                    let feasible = parsed.get("feasible").and_then(|v| v.as_bool()).unwrap_or(false);
                    yield Ok(AgentStreamEvent::Done { result: json!({ "feasibility": parsed, "state": if feasible { "PROFILE_COLLECTION" } else { "GOAL_INPUT" } }) });
                }
                Err(e) => { yield Ok(AgentStreamEvent::Error { code: "PARSE_ERROR".to_string(), message: format!("LLM response was not valid JSON: {e}") }); }
            }
        })
    }

    pub async fn collect_profile(&self, ctx: &ProfileContext) -> Result<ProfileStep, AgentError> {
        let mut vars = HashMap::new();
        vars.insert("learning_goal".to_string(), ctx.learning_goal.clone());
        vars.insert("domain".to_string(), ctx.domain.clone());
        vars.insert("answer".to_string(), ctx.answer.clone());
        vars.insert("round".to_string(), ctx.round.to_string());
        vars.insert("is_final".to_string(), ctx.is_final.to_string());
        vars.insert(
            "profile_history".to_string(),
            serde_json::to_string(&ctx.profile_history).unwrap_or_default(),
        );
        let system_prompt = self
            .prompts
            .load_and_render("profile_collection", 1, &vars)
            .map_err(AgentError::from)?;
        let user_prompt = if ctx.is_final {
            format!(
                "Final round {}/{}. Build the complete learner profile from the full profile history plus the latest answer.\nGoal: {}\nDomain: {}\nProfile history: {}\nLatest answer: {}",
                ctx.round,
                ctx.total_rounds,
                ctx.learning_goal,
                ctx.domain,
                serde_json::to_string_pretty(&ctx.profile_history).unwrap_or_default(),
                ctx.answer
            )
        } else {
            format!(
                "Profile collection round {}/{}.\nGoal: {}\nDomain: {}\nProfile history so far: {}\nLatest answer: {}\nReturn the single most useful next follow-up question as JSON.",
                ctx.round,
                ctx.total_rounds,
                ctx.learning_goal,
                ctx.domain,
                serde_json::to_string_pretty(&ctx.profile_history).unwrap_or_default(),
                ctx.answer
            )
        };
        if ctx.is_final {
            let profile = self
                .llm_json(
                    &system_prompt,
                    &user_prompt,
                    "user_profile",
                    "profile_collection",
                )
                .await?;
            Ok(ProfileStep::Complete { profile })
        } else {
            let follow_up = self
                .llm_json_unvalidated(&system_prompt, &user_prompt, "profile_followup")
                .await?;
            let next_hint = follow_up
                .get("next_question")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AgentError::JsonParse(
                        "Profile follow-up response must contain a non-empty `next_question` string"
                            .to_string(),
                    )
                })?
                .to_string();
            Ok(ProfileStep::Intermediate {
                round: ctx.round,
                total_rounds: ctx.total_rounds,
                next_question_hint: next_hint,
            })
        }
    }

    pub async fn generate_curriculum(
        &self,
        ctx: &CurriculumContext,
    ) -> Result<serde_json::Value, AgentError> {
        let mut vars = HashMap::new();
        vars.insert("learning_goal".to_string(), ctx.learning_goal.clone());
        vars.insert(
            "user_profile".to_string(),
            serde_json::to_string(&ctx.profile).unwrap_or_default(),
        );
        let system_prompt = self
            .prompts
            .load_and_render("curriculum_planning", 1, &vars)
            .map_err(AgentError::from)?;
        let user_prompt = format!(
            "Goal: {}\n\nProfile: {}",
            ctx.learning_goal,
            serde_json::to_string_pretty(&ctx.profile).unwrap_or_default()
        );
        self.llm_json(
            &system_prompt,
            &user_prompt,
            "curriculum_plan",
            "curriculum_generation",
        )
        .await
    }

    pub async fn teach_chapter(&self, ctx: &ChapterContext) -> Result<String, AgentError> {
        let (system_prompt, user_prompt) = self.chapter_prompts(ctx)?;
        self.llm_text(&system_prompt, &user_prompt, "chapter_teaching")
            .await
    }

    pub async fn repair_chapter_markdown(
        &self,
        ctx: &ChapterMarkdownRepairContext,
    ) -> Result<String, AgentError> {
        let (system_prompt, user_prompt) = self.chapter_markdown_repair_prompts(ctx)?;
        self.llm_text(&system_prompt, &user_prompt, "chapter_markdown_repair")
            .await
    }

    pub fn teach_chapter_stream(
        &self,
        ctx: ChapterContext,
    ) -> Pin<Box<dyn Stream<Item = Result<AgentStreamEvent, AgentError>> + Send>> {
        use futures::StreamExt;
        let prompts_result = self.chapter_prompts(&ctx);
        let provider = Arc::clone(&self.provider);
        let config = self.config.clone();
        let chapter_title = ctx.chapter_title.clone();
        let chapter_id = ctx.chapter_id.clone();
        let audit = self.audit.clone();

        Box::pin(async_stream::stream! {
            let (system_prompt, user_prompt) = match prompts_result { Ok(p) => p, Err(e) => { yield Err(e); return; } };
            yield Ok(AgentStreamEvent::Status { state: "CHAPTER_LEARNING".to_string(), message: format!("Loading chapter: {chapter_title}") });
            let request = make_request(&config, &system_prompt, &user_prompt, true);
            let start = Instant::now();
            let chunk_stream = provider.stream(request);
            let mut full_content = String::new();
            let mut index: u32 = 0;
            let mut chunk_stream = std::pin::pin!(chunk_stream);
            while let Some(result) = chunk_stream.next().await {
                match result {
                    Ok(chunk) => { full_content.push_str(&chunk.content); yield Ok(AgentStreamEvent::Chunk { content: chunk.content, index }); index += 1; }
                    Err(e) => { yield Ok(AgentStreamEvent::Error { code: "STREAM_ERROR".to_string(), message: e.to_string() }); return; }
                }
            }
            let duration_ms = start.elapsed().as_millis() as u64;
            if let Some(ref audit) = audit { audit.log_llm_call("stream", provider.name(), provider.model(), &Default::default(), duration_ms, true, None); }
            yield Ok(AgentStreamEvent::Done { result: json!({ "chapter_id": chapter_id, "content": full_content }) });
        })
    }

    pub async fn answer_question(&self, ctx: &QaContext) -> Result<String, AgentError> {
        let mut vars = HashMap::new();
        vars.insert("question".to_string(), ctx.question.clone());
        vars.insert(
            "user_profile".to_string(),
            serde_json::to_string(&ctx.profile).unwrap_or_default(),
        );
        vars.insert("chapter".to_string(), ctx.chapter_content.clone());
        vars.insert(
            "conversation_history".to_string(),
            ctx.conversation_history.to_string(),
        );
        vars.insert(
            "curriculum_context".to_string(),
            ctx.curriculum_context.to_string(),
        );
        let system_prompt = self
            .prompts
            .load_and_render("question_answering", 1, &vars)
            .map_err(AgentError::from)?;
        let user_prompt = format!("Question: {}", ctx.question);
        self.llm_text(&system_prompt, &user_prompt, "question_answering")
            .await
    }

    fn feasibility_prompts(
        &self,
        ctx: &FeasibilityContext,
    ) -> Result<(String, String), AgentError> {
        let mut vars = HashMap::new();
        vars.insert("learning_goal".to_string(), ctx.learning_goal.clone());
        vars.insert("domain".to_string(), ctx.domain.clone());
        vars.insert(
            "context".to_string(),
            ctx.context.clone().unwrap_or_default(),
        );
        let system_prompt = self
            .prompts
            .load_and_render("feasibility_check", 1, &vars)
            .map_err(AgentError::from)?;
        let user_prompt = format!(
            "Learning goal: {}\nDomain: {}",
            ctx.learning_goal, ctx.domain
        );
        Ok((system_prompt, user_prompt))
    }

    fn chapter_prompts(&self, ctx: &ChapterContext) -> Result<(String, String), AgentError> {
        let mut vars = HashMap::new();
        vars.insert("chapter_id".to_string(), ctx.chapter_id.clone());
        vars.insert(
            "user_profile".to_string(),
            serde_json::to_string(&ctx.profile).unwrap_or_default(),
        );
        vars.insert(
            "curriculum_context".to_string(),
            ctx.curriculum_context.to_string(),
        );
        let system_prompt = self
            .prompts
            .load_and_render("chapter_teaching", 1, &vars)
            .map_err(AgentError::from)?;
        let user_prompt = format!("Start teaching chapter: {}", ctx.chapter_title);
        Ok((system_prompt, user_prompt))
    }

    fn chapter_markdown_repair_prompts(
        &self,
        ctx: &ChapterMarkdownRepairContext,
    ) -> Result<(String, String), AgentError> {
        let mut vars = HashMap::new();
        vars.insert("chapter_id".to_string(), ctx.chapter_id.clone());
        vars.insert("chapter_title".to_string(), ctx.chapter_title.clone());
        let system_prompt = self
            .prompts
            .load_and_render("chapter_markdown_repair", 1, &vars)
            .map_err(AgentError::from)?;
        let issue_list = if ctx.issues.is_empty() {
            "- Unknown Markdown validation failure".to_string()
        } else {
            ctx.issues
                .iter()
                .map(|issue| format!("- {issue}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let user_prompt = format!(
            "Detected issues:\n{issue_list}\n\nOriginal chapter Markdown:\n```markdown\n{}\n```",
            ctx.original_markdown
        );
        Ok((system_prompt, user_prompt))
    }

    async fn llm_json(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        schema_name: &str,
        operation: &str,
    ) -> Result<serde_json::Value, AgentError> {
        let request = make_request(&self.config, system_prompt, user_prompt, false);
        let start = Instant::now();
        let response = self.provider.complete(request).await.map_err(|e| {
            if let Some(ref audit) = self.audit {
                audit.log_llm_call(
                    "unknown",
                    self.provider.name(),
                    self.provider.model(),
                    &Default::default(),
                    start.elapsed().as_millis() as u64,
                    false,
                    Some(e.to_string()),
                );
            }
            AgentError::from(e)
        })?;
        let duration_ms = start.elapsed().as_millis() as u64;
        let json_str = extract_json(&response.content);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            let truncated: String = response.content.chars().take(400).collect();
            AgentError::JsonParse(format!(
                "LLM response was not valid JSON: {e}. Raw (truncated): {truncated}"
            ))
        })?;
        self.validator
            .validate(&parsed, schema_name)
            .map_err(AgentError::from)?;
        if let Some(ref audit) = self.audit {
            audit.log_llm_call(
                "session",
                self.provider.name(),
                self.provider.model(),
                &response.usage,
                duration_ms,
                true,
                None,
            );
        }
        tracing::info!(schema = schema_name, operation = operation, model = %response.model, tokens = response.usage.total_tokens, duration_ms = duration_ms, "LLM call completed and validated");
        Ok(parsed)
    }

    async fn llm_text(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        operation: &str,
    ) -> Result<String, AgentError> {
        let request = make_request(&self.config, system_prompt, user_prompt, false);
        let start = Instant::now();
        let response = self.provider.complete(request).await.map_err(|e| {
            if let Some(ref audit) = self.audit {
                audit.log_llm_call(
                    "unknown",
                    self.provider.name(),
                    self.provider.model(),
                    &Default::default(),
                    start.elapsed().as_millis() as u64,
                    false,
                    Some(e.to_string()),
                );
            }
            AgentError::from(e)
        })?;
        let duration_ms = start.elapsed().as_millis() as u64;
        if let Some(ref audit) = self.audit {
            audit.log_llm_call(
                "session",
                self.provider.name(),
                self.provider.model(),
                &response.usage,
                duration_ms,
                true,
                None,
            );
        }
        tracing::info!(operation = operation, model = %response.model, tokens = response.usage.total_tokens, content_length = response.content.len(), duration_ms = duration_ms, "LLM text call completed");
        Ok(response.content)
    }

    async fn llm_json_unvalidated(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        operation: &str,
    ) -> Result<serde_json::Value, AgentError> {
        let request = make_request(&self.config, system_prompt, user_prompt, false);
        let start = Instant::now();
        let response = self.provider.complete(request).await.map_err(|e| {
            if let Some(ref audit) = self.audit {
                audit.log_llm_call(
                    "unknown",
                    self.provider.name(),
                    self.provider.model(),
                    &Default::default(),
                    start.elapsed().as_millis() as u64,
                    false,
                    Some(e.to_string()),
                );
            }
            AgentError::from(e)
        })?;
        let duration_ms = start.elapsed().as_millis() as u64;
        let json_str = extract_json(&response.content);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            let truncated: String = response.content.chars().take(400).collect();
            AgentError::JsonParse(format!(
                "LLM response was not valid JSON: {e}. Raw (truncated): {truncated}"
            ))
        })?;
        if let Some(ref audit) = self.audit {
            audit.log_llm_call(
                "session",
                self.provider.name(),
                self.provider.model(),
                &response.usage,
                duration_ms,
                true,
                None,
            );
        }
        tracing::info!(operation = operation, model = %response.model, tokens = response.usage.total_tokens, duration_ms = duration_ms, "LLM JSON call completed");
        Ok(parsed)
    }
}

fn make_request(
    config: &AgentConfig,
    system_prompt: &str,
    user_prompt: &str,
    stream: bool,
) -> LlmRequest {
    LlmRequest {
        model: config.provider.model.clone(),
        messages: vec![
            LlmMessage::system(system_prompt),
            LlmMessage::user(user_prompt),
        ],
        temperature: Some(config.provider.temperature),
        max_tokens: Some(config.provider.max_tokens),
        stream,
    }
}

fn extract_json(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| {
            trimmed
                .strip_prefix("```")
                .and_then(|s| s.strip_suffix("```"))
        })
    {
        return inner.trim().to_string();
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuditConfig, MemoryConfig};
    use crate::provider::mock::MockProvider;
    use crate::schema::SchemaValidator;

    fn test_config() -> AgentConfig {
        AgentConfig {
            provider: crate::config::ProviderConfig {
                provider_type: crate::config::ProviderType::Mock,
                model: "mock-model".to_string(),
                ..Default::default()
            },
            prompts_dir: std::path::PathBuf::from("../../prompts"),
            schemas_dir: std::path::PathBuf::from("../../schemas"),
            audit: AuditConfig {
                enabled: false,
                storage_dir: std::path::PathBuf::from("/tmp/blup-test-audit"),
            },
            memory: MemoryConfig {
                storage_dir: std::path::PathBuf::from("/tmp/blup-test-memory"),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    async fn test_engine_with_response(response: &str) -> AgentEngine {
        let mock = MockProvider::new();
        mock.push_response(response);
        let config = test_config();
        let prompts = Arc::new(PromptLoader::new(&config.prompts_dir));
        let validator = Arc::new(SchemaValidator::new(&config.schemas_dir));
        AgentEngine::with_provider(Arc::new(mock), prompts, validator, config).await
    }

    #[tokio::test]
    async fn test_check_feasibility() {
        let engine = test_engine_with_response(r#"{"feasible":true,"reason":"Good goal","suggestions":["Start simple"],"estimated_duration":"4 weeks","prerequisites":["Basic skills"]}"#).await;
        let result = engine
            .check_feasibility(&FeasibilityContext {
                learning_goal: "Learn Python".to_string(),
                domain: "programming".to_string(),
                context: None,
            })
            .await;
        assert!(
            result.is_ok(),
            "Feasibility check failed: {:?}",
            result.err()
        );
        assert!(result.unwrap().get("feasible").unwrap().as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_collect_profile_final() {
        let engine = test_engine_with_response(r#"{"experience_level":{"domain_knowledge":"beginner"},"learning_style":{"preferred_format":["text"],"pace_preference":"moderate"},"available_time":{"hours_per_week":10}}"#).await;
        let result = engine
            .collect_profile(&ProfileContext {
                learning_goal: "Learn Python".to_string(),
                domain: "programming".to_string(),
                answer: "beginner".to_string(),
                round: 3,
                total_rounds: 3,
                is_final: true,
                profile_history: json!([
                    {"role":"user","content":"I am new to programming"},
                    {"role":"assistant","content":"How do you like to learn?"}
                ]),
            })
            .await;
        assert!(
            result.is_ok(),
            "Profile collection failed: {:?}",
            result.err()
        );
        match result.unwrap() {
            ProfileStep::Complete { profile } => {
                assert!(profile.get("experience_level").is_some());
            }
            ProfileStep::Intermediate { .. } => panic!("Expected complete"),
        }
    }

    #[tokio::test]
    async fn test_collect_profile_intermediate() {
        let engine = test_engine_with_response(
            r#"{"next_question":"How much time can you dedicate each week?"}"#,
        )
        .await;
        let result = engine
            .collect_profile(&ProfileContext {
                learning_goal: "Learn Python".to_string(),
                domain: "programming".to_string(),
                answer: "beginner".to_string(),
                round: 1,
                total_rounds: 3,
                is_final: false,
                profile_history: json!([]),
            })
            .await;
        assert!(result.is_ok());
        match result.unwrap() {
            ProfileStep::Intermediate {
                round,
                next_question_hint,
                ..
            } => {
                assert_eq!(round, 1);
                assert_eq!(
                    next_question_hint,
                    "How much time can you dedicate each week?"
                );
            }
            ProfileStep::Complete { .. } => panic!("Expected intermediate"),
        }
    }

    #[tokio::test]
    async fn test_generate_curriculum() {
        let engine = test_engine_with_response(r#"{"title":"Learning Plan","description":"A curriculum","chapters":[{"id":"ch1","title":"Introduction","order":1,"objectives":["Basics"],"estimated_minutes":60,"prerequisites":[]}],"estimated_duration":"1 week"}"#).await;
        let result = engine.generate_curriculum(&CurriculumContext { learning_goal: "Learn Python".to_string(), profile: json!({"experience_level":{"domain_knowledge":"beginner"},"learning_style":{"preferred_format":["text"]},"available_time":{"hours_per_week":10}}) }).await;
        assert!(
            result.is_ok(),
            "Curriculum generation failed: {:?}",
            result.err()
        );
        assert!(!result
            .unwrap()
            .get("chapters")
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn test_teach_chapter() {
        let engine =
            test_engine_with_response("# Chapter 1\n\nWelcome to this chapter about variables...")
                .await;
        let result = engine
            .teach_chapter(&ChapterContext {
                chapter_id: "ch1".to_string(),
                chapter_title: "Variables".to_string(),
                profile: json!({}),
                curriculum_context: json!({}),
            })
            .await;
        assert!(
            result.is_ok(),
            "Chapter teaching failed: {:?}",
            result.err()
        );
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_repair_chapter_markdown() {
        let engine = test_engine_with_response("## Repaired\n\n- Fixed comparison").await;
        let result = engine
            .repair_chapter_markdown(&ChapterMarkdownRepairContext {
                chapter_id: "ch1".to_string(),
                chapter_title: "Logic Operators".to_string(),
                original_markdown: "| A | B |\n|---|---|\n| OR | A || B |".to_string(),
                issues: vec![
                    "line 3: Table row has 4 columns but expected 2; this usually means an unescaped `|` inside a cell".to_string(),
                ],
            })
            .await;
        assert!(result.is_ok(), "Markdown repair failed: {:?}", result.err());
        assert!(result.unwrap().contains("Repaired"));
    }

    #[tokio::test]
    async fn test_answer_question() {
        let engine =
            test_engine_with_response("A variable is a named storage location in memory...").await;
        let result = engine
            .answer_question(&QaContext {
                question: "What is a variable?".to_string(),
                chapter_content: "Chapter about variables".to_string(),
                profile: json!({}),
                conversation_history: json!([]),
                curriculum_context: json!({}),
            })
            .await;
        assert!(result.is_ok(), "Q&A failed: {:?}", result.err());
        assert!(!result.unwrap().is_empty());
    }
}
