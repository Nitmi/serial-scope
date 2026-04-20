# Release Checklist

This file is the repeatable template for the next tagged release.

## 1. Prepare changelog entry

Add a new section at the top of `CHANGELOG.md`:

```md
## [0.2.x] - YYYY-MM-DD

### Added
- ...

### Changed
- ...

### Fixed
- ...
```

Keep the version without the `v` prefix.

## 2. Bump version

Update every project-owned version reference together:

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- any UI-visible version text based on `env!("CARGO_PKG_VERSION")` is automatic once `Cargo.toml` is bumped

## 3. Run local validation

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features --locked
```

## 4. Commit release preparation

Typical split:

```bash
git add CHANGELOG.md Cargo.toml Cargo.lock README.md
git commit -m "chore(release): prepare v0.2.x"
```

If the release also depends on workflow or packaging changes, commit those first in a separate atomic commit.

## 5. Push branch and tag

```bash
git push origin main
git tag v0.2.x
git push origin v0.2.x
```

## 6. Verify GitHub Actions release run

```bash
gh run list --workflow release -R Nitmi/serial-scope --limit 5
gh run view <run-id> -R Nitmi/serial-scope
```

Expected:

- `Validate release` succeeds
- all platform `Build` jobs succeed
- `Publish release` succeeds

## 7. Verify GitHub Release output

```bash
gh release view v0.2.x -R Nitmi/serial-scope --json tagName,body,assets,url
```

Check:

- release notes body is not empty
- assets include:
  - `serial-scope-windows-x86_64-portable.exe`
  - `serial-scope-windows-x86_64-setup.exe`
  - `serial-scope-linux-x86_64.tar.gz`
  - `serial-scope-macos.app.zip`
  - `latest.json`

## 8. Verify latest.json

Download and inspect:

```bash
gh release download v0.2.x -R Nitmi/serial-scope -p latest.json -O dist/v0.2.x-latest.json
```

Verify:

- `version` matches `0.2.x`
- `notes` is not empty
- Windows asset name is `serial-scope-windows-x86_64-portable.exe`
- download URL order keeps the primary proxy first

## 9. Verify update endpoints

Check at least:

- `https://gh.123778.xyz/serial-scope/releases/latest/download/latest.json`
- `https://github.com/Nitmi/serial-scope/releases/latest/download/latest.json`

If the primary proxy is stale after release, run the purge workflow:

- workflow: `purge gh.123778 release cache`
- trigger: release `published/edited`, or manual `workflow_dispatch`

## 10. Smoke test in-app update

Recommended smoke test:

- keep one older installed build
- publish the new tag
- launch the older build
- verify it discovers the new version
- verify the update download succeeds
- verify restart succeeds

## 11. If release notes or latest.json are wrong

Fix order:

1. repair the release body:

```bash
gh release edit v0.2.x -R Nitmi/serial-scope --notes-file <notes-file>
```

2. regenerate and re-upload `latest.json`:

```bash
gh release upload v0.2.x -R Nitmi/serial-scope latest.json --clobber
```

3. manually run the purge workflow if `gh.123778.xyz` still serves stale content
