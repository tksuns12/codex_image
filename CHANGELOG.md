# Changelog

## 0.1.0 (2026-04-30)


### Features

* Added a fail-closed output writer that decodes generated images i… ([98cd004](https://github.com/tksuns12/codex_image/commit/98cd0045404cc10865d0a8838feb7722359aa3ff))
* Added a repeatable local install verifier script that installs th… ([512f4b0](https://github.com/tksuns12/codex_image/commit/512f4b00f4aada655d60cfc112647e8346e40995))
* Added an opt-in live UAT smoke script and runbook that validate l… ([21e30fc](https://github.com/tksuns12/codex_image/commit/21e30fcd470550a456b39c695b95b33a8390801c))
* Added auth lifecycle primitives for safe status inspection, idemp… ([ed92b23](https://github.com/tksuns12/codex_image/commit/ed92b2348509502e113832dbdcda2471b5bbb69b))
* Hardened diagnostics contracts with exact JSON-envelope shape che… ([7266984](https://github.com/tksuns12/codex_image/commit/7266984cf85efae51b0606790a4d9a437e90dbdb))


### Bug Fixes

* **auth:** request image generation scope ([2503731](https://github.com/tksuns12/codex_image/commit/25037318284176e6be4f0b31d663225cfb7b88d1))
* **auth:** use OAuth callback login ([57779c6](https://github.com/tksuns12/codex_image/commit/57779c68bd3bf8ce50fec3991d0ed9968403b106))
* **auth:** use OpenAI auth host for login ([5c5a099](https://github.com/tksuns12/codex_image/commit/5c5a099034ee0483e8b8e26fc6a20c64a628f2b6))
* **generate:** remove direct OpenAI backend ([1bef7e8](https://github.com/tksuns12/codex_image/commit/1bef7e860f331775faa8815d9c4d192156331147))
* **generate:** use Codex image backend by default ([6b9931f](https://github.com/tksuns12/codex_image/commit/6b9931f1d61fb28f71d73ee6b57c001530f06e2d))
* harden auth status redaction ([9f60efd](https://github.com/tksuns12/codex_image/commit/9f60efda7d7c1bccdb9e5ac7900ab3080b495f1e))

## Changelog

All notable changes to this project will be documented in this file.

This project uses release-please to update this changelog from Conventional Commit history when release PRs are prepared from the `release` branch.
