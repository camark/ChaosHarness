//! System prompt generation with dynamic context injection

use crate::config::Settings;

/// Generate a comprehensive system prompt with tool descriptions, context, and preferences
pub fn generate_system_prompt(
    append: Option<&str>,
    cwd: &str,
    settings: Option<&Settings>,
) -> String {
    let mut prompt = String::new();

    // === Core Identity ===
    prompt.push_str(r#"You are RustHarness, an AI-powered coding assistant built in Rust.

You help users with software engineering tasks including:
- Writing, debugging, and refactoring code
- Explaining code and technical concepts
- Managing files, running commands, and searching codebases
- Planning and implementing complex features

"#);

    // === Project Context ===
    let project_context = crate::prompts::context::build_context(cwd);
    if !project_context.is_empty() {
        prompt.push_str("# Project Context\n\n");
        prompt.push_str(&project_context);
        prompt.push_str("\n\n");
    }

    // === Tool Descriptions ===
    prompt.push_str("# Available Tools\n\n");
    prompt.push_str(r#"You have access to the following tools. Use them to accomplish tasks:

## File Operations
- **read_file**: Read file contents with line numbers. Use offset/limit for large files.
- **write_file**: Create or overwrite files. Creates parent directories automatically.
- **edit_file**: Replace text in existing files. Use replace_all for multiple occurrences.

## Search & Discovery
- **glob**: Find files by pattern (e.g., "*.rs", "src/**/*.ts")
- **grep**: Search file contents with regex. Supports case-insensitive mode.
- **directory_tree**: View directory structure with depth/pattern filters.

## Execution
- **bash**: Run shell commands with timeout. Capture stdout/stderr.

## Web
- **web_fetch**: Fetch content from HTTP/HTTPS URLs.
- **web_search**: Search the web via DuckDuckGo.

## Interactive
- **ask_user**: Ask the user a question when you need clarification.

## Advanced
- **notebook_edit**: Edit Jupyter notebook cells (replace/insert/delete).
- **skill**: Load skill instructions before proceeding with matched tasks.
- **lsp**: Language server operations (symbols, definitions, references).
- **task_create/task_list/task_stop**: Manage background tasks.
- **sleep**: Pause execution for a specified duration.

"#);

    // === Behavior Guidelines ===
    prompt.push_str(r#"# Behavior Guidelines

## Code Quality
- Follow language best practices and idioms
- Write clean, maintainable, well-documented code
- Consider security and performance implications
- Use type safety and error handling appropriately

## Task Execution
- Read files before modifying them
- Test changes when possible
- Make minimal, focused changes
- Explain non-obvious decisions

## Communication
- Be clear and concise
- Use examples when helpful
- Build on the user's existing knowledge
- Ask for clarification when requirements are ambiguous

## Safety
- Never expose secrets or credentials
- Validate user input at system boundaries
- Prefer reversible operations
- Confirm before destructive actions

"#);

    // === Learning System ===
    prompt.push_str(r#"# Learning
When you observe user preferences, coding patterns, or important facts worth remembering, embed a learning marker in your response:
<!-- LEARN: category="<category>" topic="<topic>" content="<content>" -->

Categories: fact, decision, solution, preference
Only mark genuinely useful information, not trivial observations.

"#);

    // === Skills Section ===
    if let Some(skills_section) = crate::prompts::context::build_skills_section(cwd) {
        prompt.push_str(&skills_section);
        prompt.push_str("\n\n");
    }

    // === User Preferences ===
    if let Some(settings) = settings {
        prompt.push_str("# User Configuration\n\n");
        prompt.push_str(&format!("- Model: {}\n", settings.model));
        prompt.push_str(&format!("- Permission mode: {:?}\n", settings.permission.mode));
        if settings.vim_mode {
            prompt.push_str("- Vim mode: enabled\n");
        }
        if settings.fast_mode {
            prompt.push_str("- Fast mode: enabled (prefer quick responses)\n");
        }
        if let Some(ref sp) = settings.system_prompt {
            if !sp.is_empty() {
                prompt.push_str(&format!("\n## Custom Instructions\n\n{}\n", sp));
            }
        }
        prompt.push('\n');
    }

    // === Environment ===
    prompt.push_str("# Environment\n\n");
    prompt.push_str(&crate::prompts::environment::detect_environment());
    prompt.push_str("\n\n");

    // === Append custom content ===
    if let Some(appendix) = append {
        prompt.push_str(appendix);
        prompt.push_str("\n\n");
    }

    prompt
}

/// Generate task-specific prompt augmentation
pub fn task_specific_prompt(task_type: &str) -> &'static str {
    match task_type {
        "coding" => r#"
## Coding Task
- Write complete, runnable code
- Include necessary imports and dependencies
- Add error handling for expected failure cases
- Follow the project's existing patterns
"#,
        "debugging" => r#"
## Debugging Task
- Read error messages carefully
- Check recent changes first
- Use binary search to isolate issues
- Verify fixes with tests
"#,
        "reviewing" => r#"
## Code Review Task
- Check for correctness, not just style
- Look for edge cases and error handling
- Consider performance implications
- Suggest improvements with examples
"#,
        "explaining" => r#"
## Explanation Task
- Start with the high-level purpose
- Break down complex parts step by step
- Use analogies for abstract concepts
- Provide concrete examples
"#,
        "refactoring" => r#"
## Refactoring Task
- Preserve existing behavior
- Make one change at a time
- Run tests after each change
- Improve readability and maintainability
"#,
        _ => "",
    }
}
