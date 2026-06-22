use crate::capture::Evidence;
use crate::store::EvidenceEntry;
use crate::WitnessError;

pub fn print_list(entries: &[EvidenceEntry], is_json: bool) -> Result<(), WitnessError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "evidence": entries,
        }))?);
    } else {
        if entries.is_empty() {
            println!("No evidence recorded yet.");
        } else {
            println!("witness: {} evidence bundle(s)", entries.len());
            println!();
            for e in entries {
                let icon = if e.exit_code == 0 { "✓" } else { "✗" };
                let tag_str = e.tag.as_deref().map(|t| format!(" [{t}]")).unwrap_or_default();
                println!("  {icon} {} `{}`{} ({}ms, exit {})",
                    e.id,
                    truncate(&e.command, 50),
                    tag_str,
                    e.duration_ms,
                    e.exit_code,
                );
            }
        }
    }
    Ok(())
}

pub fn print_evidence(evidence: &Evidence, is_json: bool) -> Result<(), WitnessError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "evidence": evidence,
        }))?);
    } else {
        let icon = if evidence.exit_code == 0 { "✓" } else { "✗" };
        println!("witness evidence: {icon} {}", evidence.id);
        println!();
        println!("  Command: {}", evidence.command);
        println!("  Timestamp: {}", evidence.timestamp);
        println!("  Duration: {}ms", evidence.duration_ms);
        println!("  Exit code: {}", evidence.exit_code);

        if let Some(ref tag) = evidence.tag {
            println!("  Tag: {tag}");
        }

        println!("  CWD: {}", evidence.cwd);
        println!("  OS: {}", evidence.environment.os);
        println!("  User: {}", evidence.environment.user);

        if let Some(ref v) = evidence.environment.rust_version {
            println!("  Rust: {v}");
        }
        if let Some(ref v) = evidence.environment.node_version {
            println!("  Node: {v}");
        }

        if let Some(ref git) = evidence.git_context {
            println!("  Git: {} @ {}{}", git.branch, git.head_sha, if git.dirty { " (dirty)" } else { "" });
        }

        println!("  Bundle hash: {}", &evidence.bundle_hash[..16]);

        if !evidence.stdout.is_empty() {
            println!();
            println!("  --- stdout ---");
            for line in evidence.stdout.lines().take(20) {
                println!("  {line}");
            }
            if evidence.stdout.lines().count() > 20 {
                println!("  ... ({} more lines)", evidence.stdout.lines().count() - 20);
            }
        }

        if !evidence.stderr.is_empty() {
            println!();
            println!("  --- stderr ---");
            for line in evidence.stderr.lines().take(10) {
                println!("  {line}");
            }
            if evidence.stderr.lines().count() > 10 {
                println!("  ... ({} more lines)", evidence.stderr.lines().count() - 10);
            }
        }
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
