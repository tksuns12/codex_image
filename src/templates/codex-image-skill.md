---
name: codex-image
description: Reusable prompt workflow for deterministic codex-image generation tasks.
---

# codex-image skill

Use this skill when you need a reproducible image-generation workflow through the `codex-image` CLI.

## Command guidance

- `codex-image generate "<prompt>" --out <dir>`
- `codex-image skill install --tool <claude|claude-code|codex|pi|opencode> --scope <global|project> --yes`
- `codex-image skill update --tool <claude|claude-code|codex|pi|opencode> --scope <global|project> --yes`

## Supported tools

- claude
- claude-code
- codex
- pi
- opencode

## Prompting guide

- https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide

## Agent checklist

1. Write the prompt in order: intended use, subject, composition, framing, viewpoint, and lighting.
2. Put any in-image words as exact text in double quotes and call out placement.
3. State constraints and invariants that must not change.
4. For multi-image inputs, index each image and add a one-line description for each index.
5. Keep outputs in project-controlled directories.
6. Run one single-change follow-up per iteration to make small, reversible edits.

## Guardrails

- Keep prompts explicit about subject, composition, lighting, and style.
- Keep outputs in project-controlled directories.
- Keep generated files under the requested `--out` directory.
- Avoid adding secrets or credentials to prompts or generated artifacts.
