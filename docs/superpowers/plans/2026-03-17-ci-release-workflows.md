# CI / Release Workflows Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** GitHub Actions で Rust CI を自動化し、`v*` タグ push 時に macOS / Windows 成果物を GitHub Releases へ自動公開できるようにする。

**Architecture:** 通常開発向けの検証は `ci.yml` に分離し、公開フローは `release.yml` でタグイベント専用に扱う。各 workflow は Rust toolchain をセットアップして標準コマンドを実行し、Release 側は OS ごとの成果物をアーカイブして同一 Release に添付する。

**Tech Stack:** GitHub Actions, Rust stable toolchain, cargo fmt, cargo clippy, cargo test, softprops/action-gh-release

---

### Task 1: Add CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`
- Test: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing test**

`ci.yml` の要件を先に定義する。最低限次を満たすことをテキストで明文化してから編集に入る。

```yaml
name: CI
on:
  push:
    branches:
      - "**"
  pull_request:

jobs:
  rust-checks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo test
```

- [ ] **Step 2: Run test to verify it fails**

Run: `test -f .github/workflows/ci.yml`
Expected: exit code `1` because the file does not exist yet

- [ ] **Step 3: Write minimal implementation**

`.github/workflows/ci.yml` を追加し、以下を含める。

- `push` / `pull_request` trigger
- `ubuntu-latest` job
- `actions/checkout@v4`
- `dtolnay/rust-toolchain@stable`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`

- [ ] **Step 4: Run test to verify it passes**

Run: `sed -n '1,220p' .github/workflows/ci.yml`
Expected: 上記要件がすべて含まれている

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add validation workflow"
```

### Task 2: Add release workflow for macOS and Windows

**Files:**
- Create: `.github/workflows/release.yml`
- Test: `.github/workflows/release.yml`

- [ ] **Step 1: Write the failing test**

Release workflow の期待仕様を先に固定する。

```yaml
name: Release
on:
  push:
    tags:
      - "v*"

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            archive_name: fee-backcalc-macos.tar.gz
          - os: windows-latest
            archive_name: fee-backcalc-windows.zip
```

さらに次を満たすことを期待する。

- `cargo build --release`
- macOS は `tar -czf`
- Windows は `Compress-Archive`
- `softprops/action-gh-release@v2` で成果物を upload

- [ ] **Step 2: Run test to verify it fails**

Run: `test -f .github/workflows/release.yml`
Expected: exit code `1` because the file does not exist yet

- [ ] **Step 3: Write minimal implementation**

`.github/workflows/release.yml` を追加し、matrix で macOS / Windows をビルドする。各ジョブで次を実装する。

- タグ `v*` を trigger にする
- stable Rust をセットアップする
- `cargo build --release` を実行する
- `target/release/fee-backcalc` または `target/release/fee-backcalc.exe` を staging directory にコピーする
- macOS は `.tar.gz`、Windows は `.zip` を作る
- `softprops/action-gh-release@v2` に生成アーカイブを渡す

- [ ] **Step 4: Run test to verify it passes**

Run: `sed -n '1,260p' .github/workflows/release.yml`
Expected: tag trigger、matrix、OS 別 archive 作成、Release upload がすべて確認できる

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release workflow"
```

### Task 3: Document release operations and verify locally

**Files:**
- Modify: `README.md`
- Test: `README.md`

- [ ] **Step 1: Write the failing test**

README に次の情報がないことを確認する。

- CI で `fmt` / `clippy` / `test` を実行する説明
- `vX.Y.Z` タグ push で Release が走る説明
- macOS / Windows 成果物名の説明

- [ ] **Step 2: Run test to verify it fails**

Run: `rg -n "clippy|GitHub Release|fee-backcalc-windows.zip|fee-backcalc-macos.tar.gz" README.md`
Expected: 必要な説明が不足しているか、結果が空になる

- [ ] **Step 3: Write minimal implementation**

`README.md` に以下を追記する。

- ローカル確認コマンドとして `cargo fmt --all -- --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test`
- GitHub Actions の CI 内容
- タグ `vX.Y.Z` push による自動 Release 作成
- macOS / Windows 成果物名と概要

- [ ] **Step 4: Run test to verify it passes**

Run: `rg -n "cargo fmt --all -- --check|cargo clippy --all-targets --all-features -- -D warnings|GitHub Releases|fee-backcalc-windows.zip|fee-backcalc-macos.tar.gz" README.md`
Expected: 追記した行が表示される

- [ ] **Step 5: Run full verification**

Run: `cargo fmt --all -- --check`
Expected: exit code `0`

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: exit code `0`

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add README.md .github/workflows/ci.yml .github/workflows/release.yml
git commit -m "docs: add ci and release workflow usage"
```
