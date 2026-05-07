# Advanced reference: agent workflows, skill lifecycle, updates, and verification

## Purpose and reader

This document is for post-first-run readers: maintainers and agent operators who already completed the quickstart and now need deterministic setup, skill lifecycle control, binary update behavior, and verification-depth choices.

After reading, you should be able to:
- install or update `codex-image` skills interactively or non-interactively
- automate skill setup safely in agent/CI contexts
- run binary update checks and apply pinned updates
- choose no-live versus live verification posture intentionally

## Skill install and update workflows

Use this section when you need to maintain `SKILL.md` installs across supported tools and scopes.

Canonical supported-tool/path/source matrix: [docs/skill-paths.md](docs/skill-paths.md)

### Interactive install (`Space` / `Enter`)

```bash
codex-image skill install
```

Interactive behavior:
- Use `Space` to toggle selections.
- Use `Enter` to confirm selections.
- Already-installed targets are preselected.
- Outdated managed installs are preselected and labeled `installed:outdated`.
- Manual/tampered installs are preselected and labeled `installed:protected`.
- Unchecking an installed managed target removes that `SKILL.md`.
- Unchecking a manual/tampered target is blocked by default; pass `--force` to allow removal.

### Deterministic install commands (agent/CI)

```bash
codex-image skill install --tool codex --tool pi --scope project --yes
codex-image skill install --tool claude-code --scope global --yes
codex-image skill install --tool opencode --scope project --yes
```

Use explicit `--tool` slugs, explicit `--scope`, and `--yes` for non-interactive automation.

### Skill updates

Interactive/default:

```bash
codex-image skill update
```

Deterministic scoped update:

```bash
codex-image skill update --tool codex --scope project --yes
```

Managed update behavior:
- Creates missing managed files.
- No-ops up-to-date managed files.
- Refreshes outdated managed files to current bundled content.
- Emits line-delimited JSON rows with stable high-level fields: `tool`, `scope`, `status`, `target_path`.
- Blocks manual/tampered files by default.
- Requires `--force` as the explicit overwrite escape hatch for blocked/tampered targets.

## Agent auto-install prompt

Use this prompt when delegating setup to an autonomous agent:

```text
Inspect the current project and choose supported tools/scopes for codex-image skills.
Run only non-interactive commands with explicit confirmation:
- codex-image skill install --tool <slug> --scope <project|global> --yes
- codex-image skill update --tool <slug> --scope <project|global> --yes
Do not mutate authentication state, do not run login flows, and do not change credentials.
Optionally run codex-image update --dry-run before any binary replacement.
```

## Binary update behavior

`codex-image update` uses GitHub Release artifacts and supports dry-run selection, non-interactive apply, and explicit version pinning.

```bash
codex-image update --dry-run
codex-image update --yes
codex-image update --version v1.2.3 --yes
```

Windows same-process replacement limitation: on Windows, do not assume in-process overwrite; prefer `codex-image update --dry-run` followed by manual replacement guidance.

## Verification posture

### No-live verification (default for routine maintenance)

Use this posture when you want contract confidence without external side effects:
- no live GitHub downloads
- no live Codex generation
- no credentials
- no auth mutation

Local install contract check (no live generation):

```bash
bash scripts/verify-local-install.sh
```

### Live smoke verification (intentional Codex-backed run)

Use this only when you explicitly want a real Codex image generation smoke test:
- Runbook: [docs/uat-live-smoke.md](docs/uat-live-smoke.md)
- Guarded command:

```bash
CODEX_IMAGE_RUN_LIVE=1 bash scripts/uat-live-smoke.sh
```

## Prompting guidance for installed skill content

When authoring or updating `SKILL.md` content, follow the OpenAI multimodal image prompting guide:
- https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide

## Related references

- Quickstart and first-run flow: `README.md`
- Canonical tool/path/source matrix: [docs/skill-paths.md](docs/skill-paths.md)
- Live Codex-backed smoke runbook: [docs/uat-live-smoke.md](docs/uat-live-smoke.md)
