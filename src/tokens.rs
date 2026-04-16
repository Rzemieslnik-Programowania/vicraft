use colored::Colorize;

#[derive(Debug, Default)]
pub struct TokenUsage {
    pub sent: Option<u64>,
    pub received: Option<u64>,
}

pub struct AiderResult {
    pub stdout: String,
    pub usage: TokenUsage,
}

pub fn parse_token_line(line: &str) -> Option<TokenUsage> {
    let trimmed = line.trim();
    if !trimmed.to_lowercase().starts_with("tokens:") {
        return None;
    }

    let after_prefix = trimmed.get(7..).unwrap_or("");
    let sent = extract_number_before(after_prefix, "sent");
    let received = extract_number_before(after_prefix, "received");

    if sent.is_some() || received.is_some() {
        Some(TokenUsage { sent, received })
    } else {
        None
    }
}

fn extract_number_before(text: &str, keyword: &str) -> Option<u64> {
    let lower = text.to_lowercase();
    let idx = lower.find(keyword)?;
    let before = &text[..idx];

    let token = before
        .split_whitespace()
        .next_back()?
        .trim_matches(|c: char| !c.is_ascii_digit() && c != ',' && c != '.' && c != 'k' && c != 'K');

    parse_number(token)
}

fn parse_number(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if s.ends_with('k') || s.ends_with('K') {
        let num: f64 = s[..s.len() - 1].parse().ok()?;
        return Some((num * 1000.0) as u64);
    }

    let stripped: String = s.chars().filter(|c| *c != ',').collect();
    stripped.parse().ok()
}

pub fn extract_usage_from_stderr(lines: &[String]) -> TokenUsage {
    lines
        .iter()
        .rev()
        .find_map(|line| parse_token_line(line))
        .unwrap_or_default()
}

pub fn display_usage(usage: &TokenUsage) {
    match (&usage.sent, &usage.received) {
        (Some(s), Some(r)) => {
            println!(
                "  {} {} sent / {} received",
                "Tokens:".dimmed(),
                format_count(*s).cyan(),
                format_count(*r).cyan()
            );
        }
        (Some(s), None) => {
            println!(
                "  {} {} sent",
                "Tokens:".dimmed(),
                format_count(*s).cyan()
            );
        }
        (None, Some(r)) => {
            println!(
                "  {} {} received",
                "Tokens:".dimmed(),
                format_count(*r).cyan()
            );
        }
        (None, None) => {
            println!("  {} {}", "Tokens:".dimmed(), "(usage data not available)".dimmed());
        }
    }
}

fn format_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_token_line() {
        let usage = parse_token_line("Tokens: 1,234 sent, 567 received.").unwrap();
        assert_eq!(usage.sent, Some(1234));
        assert_eq!(usage.received, Some(567));
    }

    #[test]
    fn parse_k_suffix() {
        let usage = parse_token_line("Tokens: 1.2k sent, 0.8k received. Cost: $0.03 message, $0.15 session.").unwrap();
        assert_eq!(usage.sent, Some(1200));
        assert_eq!(usage.received, Some(800));
    }

    #[test]
    fn parse_plain_numbers() {
        let usage = parse_token_line("Tokens: 45678 sent, 12345 received").unwrap();
        assert_eq!(usage.sent, Some(45678));
        assert_eq!(usage.received, Some(12345));
    }

    #[test]
    fn non_token_line_returns_none() {
        assert!(parse_token_line("Some random output").is_none());
    }

    #[test]
    fn extract_usage_picks_last() {
        let lines = vec![
            "Tokens: 100 sent, 50 received.".into(),
            "Some output".into(),
            "Tokens: 200 sent, 100 received.".into(),
        ];
        let usage = extract_usage_from_stderr(&lines);
        assert_eq!(usage.sent, Some(200));
        assert_eq!(usage.received, Some(100));
    }
}
