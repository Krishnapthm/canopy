# Releasing

Maintainers only. Moved from the README.

1. Set `version` in Cargo.toml, merge to main and wait for ci on main to pass.
2. Run `just pre-release` and review what it prints.
3. `git tag v<version> && git push origin v<version>`.

The release workflow builds every platform, waits for that ci run, and publishes a GitHub
Release with the archives, `SHA256SUMS` and both installers. Actions > release > Run workflow
builds everything without publishing.
