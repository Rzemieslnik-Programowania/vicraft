use anyhow::{Context, Result};
use chrono::Local;
use colored::Colorize;

use crate::aider::{self, AiderCommand};
use crate::config::Config;
use crate::git::slugify;

pub async fn run(input: &str, cfg: &Config) -> Result<()> {
    // 1. Fetch issue content
    let (task_id, issue_content) = load_issue(input, cfg).await?;
    println!("{} Loaded issue: {}", "✓".green(), task_id);

    // 2. Build output path
    std::fs::create_dir_all(".specs")?;
    let spec_path = format!(".specs/{task_id}_spec.md");

    // 3. Load context
    let spec_template = std::fs::read_to_string(".aider/templates/SPEC_TEMPLATE.md")
        .context("SPEC_TEMPLATE.md not found — run `vicraft init` first")?;
    let conventions = std::fs::read_to_string(".aider/CONVENTIONS.md").unwrap_or_default();
    let codebase = std::fs::read_to_string(".aider/context/CODEBASE.md").unwrap_or_default();

    // 4. Build prompt
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

    // 5. Run Aider in ask mode
    println!("{}", "Generating spec with Aider...".bold());
    let mut cmd = AiderCommand::ask(&cfg.aider, &prompt);

    for path in aider::default_read_files() {
        cmd = cmd.read(path);
    }
    for path in aider::relevant_skills(&issue_content) {
        cmd = cmd.read(path);
    }

    let output = cmd.run_capture()?;

    // 6. Save spec
    std::fs::write(&spec_path, &output)?;
    println!("{} Spec saved: {}", "✓".green(), spec_path.yellow());

    // 7. Summary
    println!();
    println!("{}", "Next: review the spec, answer any Open questions (section 9), then run:".bold());
    println!("   {}", format!("vicraft plan {spec_path}").cyan());

    Ok(())
}

async fn load_issue(input: &str, cfg: &Config) -> Result<(String, String)> {
    if input.to_uppercase().starts_with("LINEAR-") {
        let id = input.to_uppercase();
        let content = fetch_linear_issue(&id, &cfg.linear.api_token).await?;
        Ok((id.to_lowercase(), content))
    } else {
        let content = std::fs::read_to_string(input)
            .with_context(|| format!("Cannot read issue file: {input}"))?;
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
    if token.is_empty() {
        anyhow::bail!(
            "Linear API token not configured. Set `linear.api_token` in ~/.config/vicraft/config.toml \
             or export SPEQ_LINEAR_TOKEN."
        );
    }

    let issue_number = id.trim_start_matches("LINEAR-");
    let query = serde_json::json!({
        "query": format!(
            r#"query {{ issue(id: "{issue_number}") {{
                title description
                comments(first: 10) {{ nodes {{ body }} }}
            }} }}"#
        )
    });

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://api.linear.app/graphql")
        .header("Authorization", token)
        .json(&query)
        .send()
        .await
        .context("Linear API request failed")?
        .json()
        .await
        .context("Failed to parse Linear response")?;

    let issue = &resp["data"]["issue"];
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
