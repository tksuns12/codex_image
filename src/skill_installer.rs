const MANAGED_MARKER_PREFIX: &str = "<!-- codex-image:managed checksum=";
const MANAGED_MARKER_SUFFIX: &str = " -->";

const SKILL_BODY: &str = r#"---
name: codex-image
description: Reusable prompt workflow for deterministic codex-image generation tasks.
---

# codex-image skill

Use this skill when you need a reproducible image-generation workflow through the `codex-image` CLI.

## Command guidance

- `codex-image generate "<prompt>" --out <dir>`
- `codex-image skill install --tool <claude|claude-code|codex|pi|opencode> --scope <global|project> --yes`

## Supported tools

- claude
- claude-code
- codex
- pi
- opencode

## Prompting guide

- https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide

## Guardrails

- Keep prompts explicit about subject, composition, lighting, and style.
- Keep outputs in project-controlled directories.
- Avoid adding secrets or credentials to prompts or generated artifacts.
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillContentClassification {
    Missing,
    ManagedCurrent,
    ManagedOutdated,
    ManualUnmanaged,
    ManagedTampered,
}

pub fn render_skill_body() -> &'static str {
    SKILL_BODY
}

pub fn managed_checksum(body: &str) -> String {
    // Deterministic FNV-1a 64-bit checksum over UTF-8 bytes.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in body.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{hash:016x}")
}

pub fn managed_marker_line(body: &str) -> String {
    format!(
        "{MANAGED_MARKER_PREFIX}{}{MANAGED_MARKER_SUFFIX}",
        managed_checksum(body)
    )
}

pub fn render_managed_skill_content() -> String {
    let body = render_skill_body();
    format!("{}\n{}", managed_marker_line(body), body)
}

pub fn classify_skill_content(existing_content: Option<&str>) -> SkillContentClassification {
    let Some(existing_content) = existing_content else {
        return SkillContentClassification::Missing;
    };

    let expected_body = render_skill_body();
    let Some((existing_checksum, existing_body)) = split_managed_content(existing_content) else {
        if has_codex_image_marker_prefix(existing_content) {
            return SkillContentClassification::ManagedTampered;
        }
        return SkillContentClassification::ManualUnmanaged;
    };

    let computed = managed_checksum(existing_body);
    if computed != existing_checksum {
        return SkillContentClassification::ManagedTampered;
    }

    if existing_body == expected_body {
        SkillContentClassification::ManagedCurrent
    } else {
        SkillContentClassification::ManagedOutdated
    }
}

fn split_managed_content(content: &str) -> Option<(&str, &str)> {
    let (first_line, remainder_with_newline) = content.split_once('\n')?;

    if !first_line.starts_with(MANAGED_MARKER_PREFIX)
        || !first_line.ends_with(MANAGED_MARKER_SUFFIX)
    {
        return None;
    }

    let checksum = first_line
        .strip_prefix(MANAGED_MARKER_PREFIX)?
        .strip_suffix(MANAGED_MARKER_SUFFIX)?;

    if !is_valid_checksum_hex(checksum) {
        return None;
    }

    Some((checksum, remainder_with_newline))
}

fn has_codex_image_marker_prefix(content: &str) -> bool {
    content.lines().next().is_some_and(|line| {
        line.starts_with("<!-- codex-image:managed")
            || line.starts_with(MANAGED_MARKER_PREFIX)
            || line.contains("codex-image:managed")
    })
}

fn is_valid_checksum_hex(value: &str) -> bool {
    value.len() == 16 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
