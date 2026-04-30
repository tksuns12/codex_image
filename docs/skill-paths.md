# Supported `SKILL.md` path matrix (M002 / S01)

## Purpose

This document is the canonical support matrix for where `codex-image` skill files belong for each M002-supported tool.

S01 establishes and tests this path contract only. File-writing commands and installer UX are delivered in later slices.

## Canonical matrix

Skill name is fixed to `codex-image` and file name is fixed to `SKILL.md`.

| Tool | Global install path | Project-local install path |
| --- | --- | --- |
| Claude | `~/.claude/skills/codex-image/SKILL.md` | `.claude/skills/codex-image/SKILL.md` |
| Claude Code | `~/.claude/skills/codex-image/SKILL.md` | `.claude/skills/codex-image/SKILL.md` |
| Codex | `~/.agents/skills/codex-image/SKILL.md` | `.agents/skills/codex-image/SKILL.md` |
| pi / GSD | `~/.agents/skills/codex-image/SKILL.md` | `.agents/skills/codex-image/SKILL.md` |
| OpenCode | `~/.config/opencode/skills/codex-image/SKILL.md` | `.opencode/skills/codex-image/SKILL.md` |

## Source evidence

- Claude Code skills docs (where skills live): https://code.claude.com/docs/en/skills#where-skills-live
- Codex skills docs (where to save skills): https://developers.openai.com/codex/skills#where-to-save-skills
- OpenCode skills docs (place files): https://opencode.ai/docs/skills/#place-files
- pi / GSD skills convention: global `~/.agents/skills` and project `.agents/skills`

## Contract notes

- Codex path contract is explicitly `.agents/skills`, not `~/.codex/skills`.
- This matrix matches the hermetic resolver contract in `src/skills.rs` and `tests/skill_paths.rs`.
- Do not infer command availability from this document alone; it is a path contract reference for downstream slices.

## Prompting guide requirement for skill content

When writing `SKILL.md` content that will be installed via this matrix, follow:

- OpenAI cookbook multimodal prompting guide: https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide
