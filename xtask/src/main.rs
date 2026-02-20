use anyhow::{Context, Result, bail};
use std::path::Path;

/// Parsed check code from `define_codes!`
struct CheckCode {
    name: String,
    description: String,
    from: Option<i32>,
    until: Option<i32>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("generate-docs") => {
            let check_mode = args.iter().any(|a| a == "--check");
            generate_docs(check_mode)
        }
        _ => {
            eprintln!("Usage: cargo xtask generate-docs [--check]");
            std::process::exit(1);
        }
    }
}

fn generate_docs(check_mode: bool) -> Result<()> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let finding_rs = workspace_root.join("eksup/src/finding.rs");
    let version_rs = workspace_root.join("eksup/src/version.rs");
    let checks_md = workspace_root.join("docs/info/checks.md");

    let finding_src = std::fs::read_to_string(&finding_rs)
        .with_context(|| format!("Failed to read {}", finding_rs.display()))?;
    let version_src = std::fs::read_to_string(&version_rs)
        .with_context(|| format!("Failed to read {}", version_rs.display()))?;

    let minimum = parse_minimum(&version_src)?;
    let codes = parse_define_codes(&finding_src)?;
    let table = generate_checks_table(&codes, minimum);

    let current_content = std::fs::read_to_string(&checks_md)
        .with_context(|| format!("Failed to read {}", checks_md.display()))?;

    let new_content = splice_generated_section(&current_content, &table)?;

    if check_mode {
        if current_content != new_content {
            bail!(
                "docs/info/checks.md is out of date. Run `cargo xtask generate-docs` to update it."
            );
        }
        println!("docs/info/checks.md is up to date.");
    } else {
        std::fs::write(&checks_md, &new_content)?;
        println!("Updated docs/info/checks.md");
    }

    Ok(())
}

fn parse_minimum(version_src: &str) -> Result<i32> {
    for line in version_src.lines() {
        let line = line.trim();
        if line.starts_with("pub const MINIMUM: i32 =") {
            let value = line
                .trim_start_matches("pub const MINIMUM: i32 =")
                .trim()
                .trim_end_matches(';')
                .trim();
            return value.parse::<i32>().context("Failed to parse MINIMUM value");
        }
    }
    bail!("Could not find `pub const MINIMUM: i32 = ...` in version.rs")
}

fn parse_define_codes(src: &str) -> Result<Vec<CheckCode>> {
    let mut codes = Vec::new();
    let mut in_macro = false;

    for line in src.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("define_codes!") {
            in_macro = true;
            continue;
        }

        if !in_macro {
            continue;
        }

        // End of macro
        if trimmed == "}" {
            break;
        }

        // Match lines like: AWS001 => { desc: "...", from: None, until: None },
        if trimmed.contains("=>") && trimmed.contains("desc:") {
            let name = trimmed.split("=>").next().unwrap().trim().to_string();

            let desc = extract_quoted_value(trimmed, "desc:")
                .context(format!("Failed to parse desc for {name}"))?;
            let from = extract_option_value(trimmed, "from:");
            let until = extract_option_value(trimmed, "until:");

            codes.push(CheckCode {
                name,
                description: desc,
                from,
                until,
            });
        }
    }

    if codes.is_empty() {
        bail!("No check codes found in define_codes! macro");
    }

    Ok(codes)
}

fn extract_quoted_value(line: &str, key: &str) -> Option<String> {
    let after_key = line.split(key).nth(1)?;
    let start = after_key.find('"')? + 1;
    let end = start + after_key[start..].find('"')?;
    Some(after_key[start..end].to_string())
}

fn extract_option_value(line: &str, key: &str) -> Option<i32> {
    let after_key = line.split(key).nth(1)?;
    let trimmed = after_key.trim();
    if trimmed.starts_with("None") {
        None
    } else if trimmed.starts_with("Some(") {
        let start = 5; // len("Some(")
        let end = trimmed.find(')')?;
        trimmed[start..end].trim().parse().ok()
    } else {
        None
    }
}

fn generate_checks_table(codes: &[CheckCode], minimum: i32) -> String {
    let mut lines = Vec::new();

    lines.push("| Code | Description | Status | Applicable Versions |".to_string());
    lines.push("| :--- | :---------- | :----- | :------------------ |".to_string());

    for code in codes {
        let is_retired = code.until.map_or(false, |u| u < minimum);
        let status = if is_retired { "Retired" } else { "Active" };

        let versions = match (code.from, code.until) {
            (None, None) => "All versions".to_string(),
            (Some(f), None) => format!("1.{f}+"),
            (None, Some(u)) => format!("Up to 1.{u}"),
            (Some(f), Some(u)) => format!("1.{f} - 1.{u}"),
        };

        lines.push(format!(
            "| `{code}` | {desc} | {status} | {versions} |",
            code = code.name,
            desc = code.description,
            status = status,
            versions = versions,
        ));
    }

    lines.join("\n")
}

fn splice_generated_section(content: &str, table: &str) -> Result<String> {
    let begin_marker = "<!-- BEGIN GENERATED CHECKS TABLE -->";
    let end_marker = "<!-- END GENERATED CHECKS TABLE -->";

    let begin_pos = content
        .find(begin_marker)
        .context("Missing '<!-- BEGIN GENERATED CHECKS TABLE -->' marker in checks.md")?;
    let end_pos = content
        .find(end_marker)
        .context("Missing '<!-- END GENERATED CHECKS TABLE -->' marker in checks.md")?;

    let before = &content[..begin_pos + begin_marker.len()];
    let after = &content[end_pos..];

    Ok(format!("{before}\n\n{table}\n\n{after}"))
}
