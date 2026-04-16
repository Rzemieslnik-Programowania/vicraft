use chrono::Local;
use colored::Colorize;

use crate::aider::{self, AiderCommand};
use crate::config::Config;
use crate::error::{NetworkErrorKind, Result, VicraftError};
use crate::tokens;

pub async fn run(input: &str, cfg: &Config) -> Result<()> {
    let (task_id, issue_content) = load_issue(input, cfg).await?;
    println!("{} Loaded issue: {}", "✓".green(), task_id);

    std::fs::create_dir_all(".specs")
        .map_err(|e| VicraftError::io("spec", format!("failed to create .specs/: {e}")))?;
    let spec_path = format!(".specs/{task_id}_spec.md");

    let spec_template =
        std::fs::read_to_string(".aider/templates/SPEC_TEMPLATE.md").map_err(|_| {
            VicraftError::file_not_found(
                ".aider/templates/SPEC_TEMPLATE.md",
                "Run 'vicraft init' first",
            )
        })?;
    let conventions = std::fs::read_to_string(".aider/CONVENTIONS.md").unwrap_or_default();
    let codebase = std::fs::read_to_string(".aider/context/CODEBASE.md").unwrap_or_default();

    let date = Local::now().format("%Y-%m-%d");
    let prompt = format!(
        r#"Generate a technical specification based on the issue below.
Follow the SPEC_TEMPLATE structure exactly. Do not add or remove sections.

## Issue
{issue_content}

## Project conventions
{conventions}

## Codebase context
{codebase}

## Template
{spec_template}

## Instructions
- Fill in EVERY section of the template
- The "Out of scope" section is mandatory
- Acceptance criteria must be measurable and testable
- Apply conventions from CONVENTIONS.md
- Section 9 "Open questions": add only questions whose answers would meaningfully
  change the spec, architecture, or scope. Each must be specific and actionable.
  Format each as a checklist item: `- [ ] Question?`
  If everything is clear, leave the section body empty.
- Set {{TASK_ID}} to: {task_id}
- Set {{DATE}} to: {date}
"#
    );

    let model = cfg.model_for_step("spec");
    println!("{}", "Generating spec with Aider...".bold());
    println!("  Model: {}", model.cyan());
    let mut cmd = AiderCommand::ask(&cfg.aider, &prompt).override_model(model);

    for path in aider::default_read_files() {
        cmd = cmd.read(path);
    }
    for path in aider::relevant_skills(&issue_content) {
        cmd = cmd.read(path);
    }

    let result = cmd.run_capture()?;
    tokens::display_usage(&result.usage);
    let output = result.stdout;

    std::fs::write(&spec_path, &output)
        .map_err(|e| VicraftError::io("spec", format!("failed to write {spec_path}: {e}")))?;
    println!("{} Spec saved: {}", "✓".green(), spec_path.yellow());

    println!();
    println!(
        "{}",
        "Next: review the spec, answer any Open questions (section 9), then run:".bold()
    );
    println!("   {}", format!("vicraft plan {spec_path}").cyan());

    Ok(())
}

async fn load_issue(input: &str, cfg: &Config) -> Result<(String, String)> {
    if input.to_uppercase().starts_with("LINEAR-") {
        let id = input.to_uppercase();
        let token = if !cfg.linear.api_token.is_empty() {
            cfg.linear.api_token.clone()
        } else {
            std::env::var("SPEQ_LINEAR_TOKEN").map_err(|_| {
                VicraftError::config(
                    "~/.config/vicraft/config.toml",
                    "Linear API token not configured",
                    "Set 'linear.api_token' in ~/.config/vicraft/config.toml or export SPEQ_LINEAR_TOKEN",
                )
            })?
        };
        let content = fetch_linear_issue(&id, &token).await?;
        Ok((id.to_lowercase(), content))
    } else {
        let content = std::fs::read_to_string(input)
            .map_err(|_| VicraftError::file_not_found(input, "Check the issue file path"))?;
        let task_id = derive_task_id(input);
        Ok((task_id, content))
    }
}

fn derive_task_id(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("task")
        .to_string()
}

async fn fetch_linear_issue(id: &str, token: &str) -> Result<String> {
    let issue_number = id.trim_start_matches("LINEAR-");
    if issue_number.is_empty()
        || !issue_number
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-')
    {
        return Err(VicraftError::validation(format!(
            "Invalid Linear issue ID format: '{id}' — expected LINEAR-<identifier>"
        )));
    }

    let query = serde_json::json!({
        "query": r#"query($id: String!) {
            issue(id: $id) {
                title description
                comments(first: 10) { nodes { body } }
            }
        }"#,
        "variables": { "id": issue_number }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.linear.app/graphql")
        .header("Authorization", token)
        .json(&query)
        .send()
        .await
        .map_err(|e| classify_reqwest_error(&e))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(VicraftError::network(
            NetworkErrorKind::Auth,
            "Linear API authentication failed — check your API token",
        ));
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(VicraftError::network(
            NetworkErrorKind::RateLimit,
            "Linear API rate limit exceeded",
        ));
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(VicraftError::network(
            NetworkErrorKind::NotFound,
            format!("Linear API endpoint not found (HTTP {status})"),
        ));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| {
        VicraftError::network(
            NetworkErrorKind::Api(e.to_string()),
            "failed to parse Linear API response",
        )
    })?;

    if let Some(errors) = body.get("errors").and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            let msg: Vec<String> = errors
                .iter()
                .filter_map(|e| e["message"].as_str().map(String::from))
                .collect();
            return Err(VicraftError::network(
                NetworkErrorKind::Api(msg.join("; ")),
                "Linear GraphQL error",
            ));
        }
    }
    if body["data"]["issue"].is_null() {
        return Err(VicraftError::network(
            NetworkErrorKind::NotFound,
            format!("Linear issue not found: {id}"),
        ));
    }

    let issue = &body["data"]["issue"];
    let title = issue["title"].as_str().unwrap_or("(no title)");
    let description = issue["description"].as_str().unwrap_or("(no description)");

    let mut content = format!("# {title}\n\n{description}\n");
    if let Some(comments) = issue["comments"]["nodes"].as_array() {
        for c in comments {
            if let Some(body) = c["body"].as_str() {
                content.push_str(&format!("\n---\n{body}\n"));
            }
        }
    }

    Ok(content)
}

fn classify_reqwest_error(e: &reqwest::Error) -> VicraftError {
    if e.is_timeout() {
        return VicraftError::network(
            NetworkErrorKind::Connectivity,
            "request timed out — check your network connection",
        );
    }
    if e.is_connect() {
        return VicraftError::network(
            NetworkErrorKind::Connectivity,
            "connection failed — check your network",
        );
    }
    let status_hint = e.status().map_or("unknown".to_string(), |s| s.to_string());
    VicraftError::network(
        NetworkErrorKind::Api(format!("HTTP {status_hint}")),
        "HTTP request failed",
    )
}
