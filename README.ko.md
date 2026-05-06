# codex-image

[![Release](https://github.com/tksuns12/codex_image/actions/workflows/release.yml/badge.svg?branch=release)](https://github.com/tksuns12/codex_image/actions/workflows/release.yml)

[English](README.md)

`codex-image`는 설치된 Codex CLI에 이미지 생성을 맡기는 작은 CLI입니다. Codex의 내장 이미지 도구로 이미지를 생성한 뒤, 결과 파일을 지정한 출력 디렉터리로 복사하고 매니페스트를 작성합니다.

이 도구는 자체 OpenAI OAuth 흐름을 구현하지 않습니다. URL로 설정하는 이미지 API 엔드포인트를 호출하지 않습니다. Codex 인증 파일을 읽거나 변경하지도 않습니다. 로그인과 이미지 생성 권한은 Codex가 직접 소유합니다.

## 설치

### 릴리스 아티팩트로 설치

최신 GitHub Release에서 플랫폼에 맞는 아카이브를 내려받거나, 아래 스니펫을 사용하세요. `v0.1.0`은 설치하려는 릴리스 태그로 바꿔야 합니다.

#### Linux x86_64 / macOS x86_64 / macOS arm64

```bash
REPO="tksuns12/codex_image"
VERSION="v0.1.0"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
  Darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
  Darwin-arm64|Darwin-aarch64) TARGET="aarch64-apple-darwin" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

ASSET="codex-image-${VERSION}-${TARGET}.tar.gz"
TMPDIR="$(mktemp -d)"
curl -L "https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}" -o "${TMPDIR}/${ASSET}"
tar -xzf "${TMPDIR}/${ASSET}" -C "${TMPDIR}"
mkdir -p "${HOME}/.local/bin"
install -m 0755 "${TMPDIR}/codex-image-${VERSION}-${TARGET}/codex-image" "${HOME}/.local/bin/codex-image"

codex-image --help
```

`${HOME}/.local/bin`이 `PATH`에 포함되어 있어야 합니다.

#### Windows x86_64 PowerShell

```powershell
$Repo = "tksuns12/codex_image"
$Version = "v0.1.0"
$Target = "x86_64-pc-windows-msvc"
$Asset = "codex-image-$Version-$Target.zip"
$TempDir = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP "codex-image-install")
$ZipPath = Join-Path $TempDir $Asset

Invoke-WebRequest "https://github.com/$Repo/releases/download/$Version/$Asset" -OutFile $ZipPath
Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force
New-Item -ItemType Directory -Force -Path "$HOME\bin" | Out-Null
Copy-Item "$TempDir\codex-image-$Version-$Target\codex-image.exe" "$HOME\bin\codex-image.exe" -Force

codex-image --help
```

`$HOME\bin`이 `PATH`에 포함되어 있어야 합니다.

### 소스에서 설치

로컬 개발을 하거나, 게시된 릴리스가 아니라 현재 체크아웃을 의도적으로 설치할 때 사용합니다.

```bash
cargo install --path . --force
codex-image --help
```

## 사전 요구 사항: Codex CLI

`codex-image generate`는 정상 동작하는 Codex 설치가 필요합니다. Codex 실행 파일은 다음 순서로 찾습니다.

1. `CODEX_IMAGE_CODEX_BIN`이 설정되어 있으면 그 값을 사용합니다.
2. `PATH`에 있는 `codex`를 사용합니다.
3. VS Code/Cursor 확장에서 흔히 쓰이는 Codex 설치 위치를 확인합니다.

Codex는 이미 로그인되어 있어야 하며, 내장 이미지 생성 도구를 사용할 수 있어야 합니다.

## 이미지와 매니페스트 생성

출력 디렉터리를 지정해 생성을 실행합니다.

```bash
codex-image generate "도서관에서 책을 읽는 수채화풍 여우" --out ./out
```

이 명령은 다음 작업을 수행합니다.

1. `codex exec`를 실행합니다.
2. Codex에 내장 이미지 생성 도구 사용을 지시합니다.
3. 생성된 이미지 경로가 담긴 Codex의 최종 JSON 응답을 읽습니다.
4. 생성된 파일을 `--out` 아래 `image-0001.<format>` 이름으로 복사합니다.
5. `--out` 아래 `manifest.json`을 작성합니다.
6. 매니페스트 JSON을 stdout으로 출력합니다.

stdout 예시 형식:

```json
{
  "prompt": "도서관에서 책을 읽는 수채화풍 여우",
  "model": "gpt-image-2",
  "manifest_path": "./out/manifest.json",
  "images": [
    {
      "index": 1,
      "path": "./out/image-0001.png",
      "format": "png",
      "byte_count": 12345
    }
  ],
  "response": {
    "created": 1777523488,
    "usage": {}
  }
}
```

## 에이전트 스킬 설치/업데이트 가이드

독자: 이 저장소를 처음 보는 사람 또는 AI 에이전트.
읽은 뒤 수행할 행동: 지원 도구/스코프를 고르고 `codex-image skill install`/`codex-image skill update`를 재현 가능한 방식으로 실행하며, 필요 시 바이너리 업데이트를 안전하게 검증합니다.

### 지원 도구 매트릭스

정식 경로/근거 문서: [docs/skill-paths.md](docs/skill-paths.md)

| Tool | CLI `--tool` slug | Global scope path | Project scope path |
| --- | --- | --- | --- |
| Claude | `claude` | `~/.claude/skills/codex-image/SKILL.md` | `.claude/skills/codex-image/SKILL.md` |
| Claude Code | `claude-code` | `~/.claude/skills/codex-image/SKILL.md` | `.claude/skills/codex-image/SKILL.md` |
| Codex | `codex` | `~/.agents/skills/codex-image/SKILL.md` | `.agents/skills/codex-image/SKILL.md` |
| pi | `pi` | `~/.agents/skills/codex-image/SKILL.md` | `.agents/skills/codex-image/SKILL.md` |
| OpenCode | `opencode` | `~/.config/opencode/skills/codex-image/SKILL.md` | `.opencode/skills/codex-image/SKILL.md` |

### `codex-image skill install`

TTY 상호작용 설치:

```bash
codex-image skill install
```

선택 토글은 `Space`, 확정은 `Enter`를 사용합니다.

에이전트/CI용 고정 명령:

```bash
codex-image skill install --tool codex --tool pi --scope project --yes
codex-image skill install --tool claude-code --scope global --yes
```

### `codex-image skill update`

기본 실행:

```bash
codex-image skill update
```

비대화식 범위 지정 예시:

```bash
codex-image skill update --tool codex --scope project --yes
```

관리 대상 업데이트 동작:
- 누락 파일은 생성(create)
- 최신 파일은 변경 없음(no-op)
- 구버전 관리 파일은 갱신(refresh)
- line-delimited JSON 행에 `tool`, `scope`, `status`, `target_path` 필드 제공
- 수동 수정/변조 파일은 기본 차단
- `--force`로만 명시적 덮어쓰기 허용

### Agent auto-install prompt

아래 프롬프트를 그대로 복사해 사용할 수 있습니다.

```text
Inspect the current project and choose supported tools/scopes for codex-image skills.
Run only non-interactive commands with explicit confirmation:
- codex-image skill install --tool <slug> --scope <project|global> --yes
- codex-image skill update --tool <slug> --scope <project|global> --yes
Do not mutate authentication state, do not run login flows, and do not change credentials.
Optionally run codex-image update --dry-run before any binary replacement.
```

### 바이너리 업데이트

`codex-image update`는 GitHub Release artifacts를 사용하며 dry-run/자동 적용/버전 고정을 지원합니다.

```bash
codex-image update --dry-run
codex-image update --yes
codex-image update --version v1.2.3 --yes
```

Windows same-process replacement limitation 이 있으므로, Windows에서는 `codex-image update --dry-run` 후 수동 교체 가이드를 따르세요.

검증 원칙(라이브 의존성 없음):
- GitHub 다운로드를 라이브로 수행하지 않습니다 (no live GitHub downloads)
- Codex 생성은 라이브로 수행하지 않습니다 (no live Codex generation)
- 자격 증명을 요구하지 않습니다 (no credentials)
- 인증 상태를 변경하지 않습니다 (no auth mutation)

## 환경 변수

- `CODEX_IMAGE_CODEX_BIN`은 선택 사항이며, Codex 실행 파일 경로를 지정합니다.

URL 기반 환경 변수는 지원하지 않습니다. 별도의 인증/API 동작도 없습니다.

## 릴리스 워크플로

릴리스는 `release` 브랜치에서만 생성됩니다.

릴리스 워크플로는 release-please를 사용해 Conventional Commit 메시지로 SemVer를 결정합니다.

- `fix:`는 패치 릴리스를 만듭니다.
- `feat:`는 마이너 릴리스를 만듭니다.
- `feat!:`, `fix!:` 또는 `!`가 붙은 breaking-change 커밋은 메이저 릴리스를 만듭니다.

`release` 브랜치 보호 권장 설정:

- 병합 전 pull request를 요구합니다.
- `Release / Preflight` 상태 체크를 요구합니다.
- 병합 전 브랜치 최신화를 요구합니다.
- 저장소 정책이 허용하면 직접 push를 제한합니다.

`release` 브랜치 대상 pull request에서는 워크플로가 테스트와 clippy를 실행합니다. `release` 브랜치에 push되면 release-please가 릴리스 PR을 열거나 갱신합니다. 그 릴리스 PR이 병합되면 GitHub Release가 생성되고, 워크플로가 Linux, macOS, Windows용 아카이브를 빌드해 업로드합니다.

## 검증 스크립트

### 로컬 설치 검증

```bash
bash scripts/verify-local-install.sh
```

이 스크립트는 실제 이미지 생성을 요구하지 않고 `cargo install --path .`, 설치된 바이너리 실행, help/usage 동작을 검증합니다.

### Live UAT smoke

실제 Codex 기반 이미지 생성을 의도적으로 확인할 때만 실행합니다.

```bash
CODEX_IMAGE_RUN_LIVE=1 bash scripts/uat-live-smoke.sh
```

라이브 스크립트는 보호되어 있으며, `CODEX_IMAGE_RUN_LIVE=1`이 설정되지 않으면 즉시 종료합니다.

사용 전 전체 런북을 읽으세요: [docs/uat-live-smoke.md](docs/uat-live-smoke.md)
