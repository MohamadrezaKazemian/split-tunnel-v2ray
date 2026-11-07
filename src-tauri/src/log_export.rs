//! Log-export formatting and conservative redaction.
use regex::Regex;

pub struct LogExport<'a> {
    pub app_version: &'a str,
    pub os: &'a str,
    pub engine_version: &'a str,
    pub mode: &'a str,
    pub rule_count: usize,
    pub lines: &'a [String],
    pub include_sensitive: bool,
}

pub fn format(export: LogExport<'_>) -> String {
    let logs = export.lines.join("\n");
    let logs = if export.include_sensitive {
        logs
    } else {
        redact(&logs)
    };
    format!(
        "Split Tunnel log\nApp version: {}\nOS: {}\nEngine version: {}\nTunnel mode: {}\nSelected app rules: {}\n\nLogs:\n{}\n",
        export.app_version, export.os, export.engine_version, export.mode, export.rule_count, logs
    )
}

pub fn redact(value: &str) -> String {
    let mut redacted = value.to_owned();
    for (pattern, replacement) in [
        (
            r"(?i)\b(?:vless|vmess|trojan|ss)://[^\s]+",
            "[redacted proxy URL]",
        ),
        (r"(?i)\bhttps?://[^\s]+", "[redacted URL]"),
        (
            r#"(?i)(\"(?:password|uuid|server|host|sni)\"\s*:\s*\")[^\"]*"#,
            "$1<redacted>",
        ),
        (
            r"(?i)\b(?:password|uuid|server|host|sni)\s*[=:]\s*[^\s,]+",
            "[redacted connection detail]",
        ),
        (r"(?i)\b(?:[a-z0-9-]+\.)+[a-z]{2,}\b", "<redacted host>"),
        (
            r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b",
            "<redacted UUID>",
        ),
        (r"\b(?:\d{1,3}\.){3}\d{1,3}\b", "<redacted IP>"),
        (r"/home/[^/\s]+", "/home/<user>"),
        (r"(?i)[a-z]:\\\\users\\\\[^\\\s]+", r"C:\\Users\\<user>"),
    ] {
        redacted = Regex::new(pattern)
            .expect("valid redaction regex")
            .replace_all(&redacted, replacement)
            .into_owned();
    }
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_connection_details_and_home_paths() {
        let source = "vless://secret@example.invalid:443 subscription https://example.invalid/list?token=abc dial plain.example.invalid:443 uuid=11111111-2222-4333-8444-555555555555 server=203.0.113.8 password=hunter2 /home/alice/.config/app C:\\\\Users\\\\Alice\\\\AppData";
        let output = redact(source);
        for secret in [
            "secret",
            "example.invalid",
            "plain.example.invalid",
            "203.0.113.8",
            "hunter2",
            "alice",
            "Alice",
            "11111111",
        ] {
            assert!(!output.contains(secret), "leaked {secret}");
        }
        assert!(output.contains("[redacted proxy URL]"));
        assert!(output.contains("/home/<user>"));
    }

    #[test]
    fn sensitive_details_are_opt_in() {
        let lines = vec!["server=example.invalid".to_owned()];
        let output = format(LogExport {
            app_version: "0",
            os: "linux",
            engine_version: "1",
            mode: "whole",
            rule_count: 0,
            lines: &lines,
            include_sensitive: false,
        });
        assert!(!output.contains("example.invalid"));
        let output = format(LogExport {
            app_version: "0",
            os: "linux",
            engine_version: "1",
            mode: "whole",
            rule_count: 0,
            lines: &lines,
            include_sensitive: true,
        });
        assert!(output.contains("example.invalid"));
    }
}
