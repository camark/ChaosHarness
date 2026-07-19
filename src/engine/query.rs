//! Query engine with tool-use loop support

#![allow(dead_code)]

use crate::api::client::{ApiClient, ApiRequest, ApiUsage};
use crate::api::errors::ApiError;
use crate::config::Settings;
use crate::engine::messages::{
    assistant_message_from_api, ConversationMessage, MessageContent, MessageRole, ToolResultBlock, ToolUseData,
};
use crate::hooks::executor::HookExecutor;
use crate::hooks::registry::HookRegistry;
use crate::learning::retriever::ContextRetriever;
use crate::learning::store::KnowledgeStore;
use crate::learning::summarizer::SmartCompactor;
use crate::learning::extractor::LearningEngine;
use crate::permissions::checker::PermissionChecker;
use crate::tools::base::{ToolExecutionContext, ToolRegistry};
use crate::mcp::client::McpManager;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Usage tracking
#[derive(Debug, Clone, Default)]
pub struct UsageTracker {
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
}

impl UsageTracker {
    pub fn add(&mut self, usage: &ApiUsage) {
        self.total_input_tokens += usage.input_tokens;
        self.total_output_tokens += usage.output_tokens;
    }
}

/// Query engine for conversation with tool support
pub struct QueryEngine {
    client: ApiClient,
    tool_registry: ToolRegistry,
    permission_checker: PermissionChecker,
    hook_executor: HookExecutor,
    settings: Settings,
    cwd: PathBuf,
    messages: Arc<Mutex<Vec<ConversationMessage>>>,
    usage: Arc<Mutex<UsageTracker>>,
    max_turns: usize,
    use_openai_format: bool,
    mcp_manager: Option<Arc<McpManager>>,
    pub context_retriever: Option<ContextRetriever>,
    pub smart_compactor: Option<SmartCompactor>,
    pub learning_engine: Option<LearningEngine>,
}

impl QueryEngine {
    pub fn new(settings: Settings, tool_registry: ToolRegistry, cwd: PathBuf) -> Result<Self, String> {
        let api_key = settings.resolve_api_key()?;
        let client = ApiClient::new(api_key.clone(), settings.base_url.clone());

        let permission_checker = PermissionChecker::new(settings.permission.clone());

        // Initialize hooks from settings
        let hook_registry = HookRegistry::new();
        for hook_def in &settings.hooks.hooks {
            hook_registry.register_blocking(hook_def.clone());
        }
        let hook_executor = HookExecutor::new(hook_registry);

        // Detect if we should use OpenAI format
        let use_openai_format = settings.base_url.as_ref().is_some_and(|url| {
            url.contains("moonshot") || url.contains("openai")
        }) || api_key.starts_with("sk-");

        // Build system prompt with full context
        let cwd_str = cwd.to_str().unwrap_or(".");
        let system_prompt = if settings.system_prompt.is_some() {
            // User provided custom prompt - use it with context augmentation
            let mut prompt = settings.system_prompt.clone().unwrap();
            let context = crate::prompts::context::build_context(cwd_str);
            if !context.is_empty() {
                prompt.push_str("\n\n# Project Context\n\n");
                prompt.push_str(&context);
            }
            if let Some(skills_section) = crate::prompts::context::build_skills_section(cwd_str) {
                prompt.push_str("\n\n");
                prompt.push_str(&skills_section);
            }
            Some(prompt)
        } else {
            // Generate comprehensive system prompt
            Some(crate::prompts::system_prompt::generate_system_prompt(
                None,
                cwd_str,
                Some(&settings),
            ))
        };

        // Create mutable settings clone to update system_prompt
        let mut settings = settings;
        settings.system_prompt = system_prompt;

        // Initialize MCP manager if servers are configured
        let mcp_manager: Option<Arc<McpManager>> = None; // Will be initialized below

        // Initialize learning system if enabled
        let (context_retriever, smart_compactor, learning_engine) = if settings.learning.enabled {
            let db_path = settings.learning.knowledge_db_path.as_ref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    cwd.join(".rust_harness").join("knowledge.db")
                });

            // Ensure parent directory exists
            if let Some(parent) = db_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            match KnowledgeStore::new(&db_path) {
                Ok(store) => {
                    let retriever = ContextRetriever::new(
                        store,
                        settings.learning.bm25_k1,
                        settings.learning.bm25_b,
                        settings.learning.bm25_top_k,
                        settings.learning.max_context_injection_tokens,
                    );

                    let compactor = SmartCompactor::new(
                        ApiClient::new(api_key.clone(), settings.base_url.clone()),
                        settings.model.clone(),
                        settings.learning.summary_segment_size,
                        settings.learning.summary_token_threshold,
                    );

                    let extractor = LearningEngine::new(
                        KnowledgeStore::new(&db_path).unwrap(),
                        settings.learning.bm25_k1,
                        settings.learning.bm25_b,
                        Some(ApiClient::new(api_key.clone(), settings.base_url.clone())),
                        settings.model.clone(),
                        settings.learning.session_end_extraction,
                        cwd.clone(),
                    );

                    (Some(retriever), Some(compactor), Some(extractor))
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize learning system: {}", e);
                    (None, None, None)
                }
            }
        } else {
            (None, None, None)
        };

        Ok(Self {
            client,
            tool_registry,
            permission_checker,
            hook_executor,
            settings,
            cwd,
            messages: Arc::new(Mutex::new(Vec::new())),
            usage: Arc::new(Mutex::new(UsageTracker::default())),
            max_turns: 200,
            use_openai_format,
            mcp_manager,
            context_retriever,
            smart_compactor,
            learning_engine,
        })
    }

    /// Initialize MCP connections and register MCP tools
    pub async fn initialize_mcp(&mut self) -> Vec<String> {
        // Create MCP manager from settings
        let manager = McpManager::new(self.settings.clone());

        // Connect to all configured servers
        let connected_servers = manager.initialize_all().await.unwrap_or_else(|e| {
            tracing::error!("Failed to initialize MCP manager: {}", e);
            Vec::new()
        });

        // Register MCP tools if any servers connected
        if !connected_servers.is_empty() {
            let mcp_manager = Arc::new(manager);

            // Register MCP tools with the tool registry
            crate::tools::mcp::register_mcp_tools(mcp_manager.clone(), &self.tool_registry).await;

            self.mcp_manager = Some(mcp_manager);
            tracing::info!("Initialized {} MCP servers", connected_servers.len());
        }

        connected_servers
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }

    /// Get current messages
    pub async fn get_messages(&self) -> Vec<ConversationMessage> {
        self.messages.lock().await.clone()
    }

    /// Set messages (for session resume)
    pub async fn set_messages(&self, messages: Vec<ConversationMessage>) {
        let mut msgs = self.messages.lock().await;
        *msgs = messages;
    }

    /// Load messages from JSON values (for session resume)
    pub async fn load_messages(&self, messages: Vec<serde_json::Value>) {
        let mut msgs = self.messages.lock().await;
        msgs.clear();
        for msg in messages {
            if let Ok(conv_msg) = serde_json::from_value::<ConversationMessage>(msg) {
                msgs.push(conv_msg);
            }
        }
    }

    /// Get total usage
    pub async fn get_usage(&self) -> UsageTracker {
        self.usage.lock().await.clone()
    }

    /// Clear conversation
    pub async fn clear(&self) {
        self.messages.lock().await.clear();
        *self.usage.lock().await = UsageTracker::default();
    }

    /// Send a message and run the tool-use loop
    pub async fn send_message(&self, prompt: String) -> Result<String, ApiError> {
        // Add user message
        {
            let mut messages = self.messages.lock().await;
            messages.push(ConversationMessage::user_text(prompt));
        }

        self.run_loop().await
    }

    /// Run the tool-use loop
    async fn run_loop(&self) -> Result<String, ApiError> {
        let tools_schema = self.tool_registry.to_api_schema().await;

        for turn in 0..self.max_turns {
            // Auto-compact if needed
            {
                let messages = self.messages.lock().await;
                let (compacted, was_compacted) = crate::engine::compact::auto_compact_if_needed(messages.clone());
                if was_compacted {
                    drop(messages);
                    let mut msgs = self.messages.lock().await;
                    *msgs = compacted;
                    tracing::info!("Auto-compacted message history");
                }
            }

            // Retrieve relevant context from knowledge base
            let mut enriched_system_prompt = self.settings.system_prompt.clone();

            if let Some(ref retriever) = self.context_retriever {
                let current_query = {
                    let messages = self.messages.lock().await;
                    messages.last()
                        .filter(|m| m.role == MessageRole::User)
                        .and_then(|m| m.content.first())
                        .and_then(|c| match c {
                            MessageContent::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .unwrap_or_default()
                };

                if !current_query.is_empty() {
                    if let Ok(contexts) = retriever.retrieve(&current_query) {
                        let context_section = retriever.format_for_prompt(&contexts);
                        if !context_section.is_empty() {
                            enriched_system_prompt = Some(match enriched_system_prompt {
                                Some(mut prompt) => {
                                    prompt.push_str("\n\n");
                                    prompt.push_str(&context_section);
                                    prompt
                                }
                                None => context_section,
                            });
                        }
                    }
                }
            }

            let messages = self.messages.lock().await.clone();

            // Call API
            let request = ApiRequest {
                model: self.settings.model.clone(),
                messages,
                system_prompt: enriched_system_prompt,
                max_tokens: self.settings.max_tokens,
                tools: tools_schema.clone(),
            };

            let response = self.client.send_message(request).await?;

            // Track usage
            {
                let mut usage = self.usage.lock().await;
                usage.add(&response.usage);
            }

            // Build assistant message
            let assistant_msg = assistant_message_from_api(
                &response
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        MessageContent::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(""),
                response.tool_uses.clone(),
            );

            // Add to messages
            {
                let mut messages = self.messages.lock().await;
                messages.push(assistant_msg);
            }

            // Check if model wants to use tools
            if response.tool_uses.is_empty() {
                // No tools, we're done - return the text response
                let text = response
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        MessageContent::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                return Ok(text);
            }

            // Execute tools
            let tool_results = self
                .execute_tools(response.tool_uses, turn)
                .await;

            // Add tool results to messages
            {
                let mut messages = self.messages.lock().await;
                if self.use_openai_format {
                    // OpenAI format: each tool result is a separate message
                    for result in tool_results {
                        messages.push(ConversationMessage {
                            role: MessageRole::User,
                            content: vec![MessageContent::ToolResult {
                                tool_use_id: result.tool_use_id,
                                content: result.content,
                                is_error: result.is_error,
                            }],
                            tool_uses: Vec::new(),
                        });
                    }
                } else {
                    // Anthropic format: all tool results in one message
                    messages.push(ConversationMessage::tool_results(tool_results));
                }
            }
        }

        Err(ApiError::Request(format!(
            "Exceeded maximum turn limit ({})",
            self.max_turns
        )))
    }

    /// Execute multiple tools, potentially in parallel
    async fn execute_tools(
        &self,
        tool_uses: Vec<ToolUseData>,
        _turn: usize,
    ) -> Vec<ToolResultBlock> {
        let mut results = Vec::new();

        // Execute tools in parallel if there are multiple
        if tool_uses.len() == 1 {
            let tool_use = tool_uses.into_iter().next().unwrap();
            let result = self.execute_single_tool(&tool_use).await;
            results.push(result);
        } else {
            // Execute multiple tools concurrently
            let mut handles = Vec::new();
            for tool_use in &tool_uses {
                let handle = tokio::spawn({
                    let this = self.clone();
                    let tool_use = tool_use.clone();
                    async move { this.execute_single_tool(&tool_use).await }
                });
                handles.push(handle);
            }

            for (tool_use, handle) in tool_uses.iter().zip(handles) {
                match handle.await {
                    Ok(result) => results.push(result),
                    Err(e) => results.push(ToolResultBlock {
                        tool_use_id: tool_use.id.clone(),
                        content: format!("Task failed: {}", e),
                        is_error: true,
                    }),
                }
            }
        }

        results
    }

    /// Execute a single tool with permission checking, hooks, and error recovery
    async fn execute_single_tool(&self, tool_use: &ToolUseData) -> ToolResultBlock {
        tracing::info!("Executing tool: {} with input: {:?}", tool_use.name, tool_use.input);

        // Execute pre_tool_use hooks
        if let Some(block_reason) = self.hook_executor
            .check_pre_tool_use(&tool_use.name, &tool_use.input)
            .await
        {
            tracing::warn!("Tool {} blocked by hook: {}", tool_use.name, block_reason);
            return ToolResultBlock {
                tool_use_id: tool_use.id.clone(),
                content: block_reason,
                is_error: true,
            };
        }

        // Get the tool
        let tool = match self.tool_registry.get(&tool_use.name).await {
            Some(t) => t,
            None => {
                // Suggest similar tools on unknown tool
                let suggestion = self.suggest_similar_tool(&tool_use.name);
                let msg = if let Some(suggestion) = suggestion {
                    format!("Unknown tool: {}. Did you mean '{}'?", tool_use.name, suggestion)
                } else {
                    format!("Unknown tool: {}", tool_use.name)
                };
                return ToolResultBlock {
                    tool_use_id: tool_use.id.clone(),
                    content: msg,
                    is_error: true,
                };
            }
        };

        // Permission check
        let is_read_only = tool.is_read_only(&tool_use.input);
        let decision = self.permission_checker.evaluate(
            &tool_use.name,
            is_read_only,
            crate::permissions::checker::PermissionChecker::extract_file_path(&tool_use.input).as_deref(),
            None,
        ).await;

        if !decision.allowed {
            return ToolResultBlock {
                tool_use_id: tool_use.id.clone(),
                content: format!("Permission denied: {}", decision.reason),
                is_error: true,
            };
        }

        // Execute the tool with retry logic for transient errors
        let context = ToolExecutionContext::new(self.cwd.clone());
        let max_retries = 1; // One retry for transient errors

        let mut last_error = None;
        for attempt in 0..=max_retries {
            match tool.execute(tool_use.input.clone(), context.clone()).await {
                Ok(result) => {
                    let tool_result = ToolResultBlock {
                        tool_use_id: tool_use.id.clone(),
                        content: result.output.clone(),
                        is_error: result.is_error,
                    };

                    // Execute post_tool_use hooks
                    self.hook_executor.notify_post_tool_use(
                        &tool_use.name,
                        &tool_use.input,
                        &result.output,
                        result.is_error,
                    ).await;

                    // Track tool usage
                    self.track_tool_usage(&tool_use.name, result.is_error).await;

                    return tool_result;
                }
                Err(e) => {
                    let error_msg = format!("{}", e);

                    // Check if this is a retryable error (timeout, network, transient)
                    let is_retryable = error_msg.contains("timeout")
                        || error_msg.contains("timed out")
                        || error_msg.contains("connection")
                        || error_msg.contains("network");

                    if is_retryable && attempt < max_retries {
                        tracing::warn!("Tool {} failed (attempt {}), retrying: {}", tool_use.name, attempt + 1, error_msg);
                        last_error = Some(error_msg);
                        continue;
                    }

                    // Notify error hooks
                    self.hook_executor.notify_error(&format!("Tool execution error: {}", e)).await;

                    // Track failed tool usage
                    self.track_tool_usage(&tool_use.name, true).await;

                    // Provide helpful error context
                    let error_context = self.provide_error_context(&tool_use.name, &error_msg);

                    return ToolResultBlock {
                        tool_use_id: tool_use.id.clone(),
                        content: error_context,
                        is_error: true,
                    };
                }
            }
        }

        // Should not reach here, but handle it
        ToolResultBlock {
            tool_use_id: tool_use.id.clone(),
            content: format!("Tool failed after retries: {}", last_error.unwrap_or_default()),
            is_error: true,
        }
    }

    /// Suggest similar tool name when unknown tool is used
    fn suggest_similar_tool(&self, name: &str) -> Option<String> {
        let tools = self.tool_registry.list_names_sync();
        let mut best_match = None;
        let mut best_score: f64 = 0.0;

        for tool_name in &tools {
            let score = similarity_score(name, tool_name);
            if score > best_score && score > 0.5 {
                best_score = score;
                best_match = Some(tool_name.clone());
            }
        }

        best_match
    }

    /// Provide contextual error information
    fn provide_error_context(&self, tool_name: &str, error: &str) -> String {
        let mut context = format!("Tool execution error: {}", error);

        // Add hints for common errors
        if error.contains("No such file") || error.contains("not found") {
            context.push_str("\n\nHint: Check if the file path is correct and the file exists.");
        } else if error.contains("Permission denied") {
            context.push_str("\n\nHint: Check file permissions or run with appropriate privileges.");
        } else if error.contains("timeout") || error.contains("timed out") {
            context.push_str("\n\nHint: The operation took too long. Try with a smaller scope or increase timeout.");
        } else if tool_name == "bash" && error.contains("command not found") {
            context.push_str("\n\nHint: The command may not be installed or not in PATH.");
        }

        context
    }

    /// Track tool usage statistics
    async fn track_tool_usage(&self, tool_name: &str, is_error: bool) {
        // Log tool usage for learning
        if is_error {
            tracing::warn!("Tool '{}' execution failed", tool_name);
        } else {
            tracing::debug!("Tool '{}' executed successfully", tool_name);
        }
    }
}

impl Clone for QueryEngine {
    fn clone(&self) -> Self {
        Self {
            client: ApiClient::new(
                self.settings.resolve_api_key().unwrap_or_default(),
                self.settings.base_url.clone(),
            ),
            tool_registry: self.tool_registry.clone(),
            permission_checker: self.permission_checker.clone(),
            hook_executor: self.hook_executor.clone(),
            settings: self.settings.clone(),
            cwd: self.cwd.clone(),
            messages: self.messages.clone(),
            usage: self.usage.clone(),
            max_turns: self.max_turns,
            use_openai_format: self.use_openai_format,
            mcp_manager: self.mcp_manager.clone(),
            context_retriever: None,
            smart_compactor: None,
            learning_engine: None,
        }
    }
}

/// Simple string similarity score (Levenshtein-based)
fn similarity_score(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    // Exact match
    if a_lower == b_lower {
        return 1.0;
    }

    // Check if one contains the other
    if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
        return 0.8;
    }

    // Check prefix match
    let min_len = a_lower.len().min(b_lower.len());
    if min_len > 0 {
        let common_prefix: usize = a_lower.chars()
            .zip(b_lower.chars())
            .take_while(|(a, b)| a == b)
            .count();

        if common_prefix >= 3 {
            return 0.6 + (common_prefix as f64 / min_len as f64) * 0.2;
        }
    }

    // Levenshtein distance
    let len_a = a_lower.len();
    let len_b = b_lower.len();
    let mut matrix = vec![vec![0usize; len_b + 1]; len_a + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(len_a + 1) {
        row[0] = i;
    }
    for (j, val) in matrix[0].iter_mut().enumerate().take(len_b + 1) {
        *val = j;
    }

    for (i, ca) in a_lower.chars().enumerate() {
        for (j, cb) in b_lower.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }

    let max_len = len_a.max(len_b) as f64;
    if max_len == 0.0 {
        return 0.0;
    }

    1.0 - (matrix[len_a][len_b] as f64 / max_len)
}
