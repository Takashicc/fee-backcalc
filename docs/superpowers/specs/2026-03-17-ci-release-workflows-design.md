# CI / Release Workflows Design

## Summary

`fee-backcalc` に GitHub Actions ベースの CI と Release ワークフローを追加する。
日常の品質確認と配布処理を分離し、通常の開発では高速にフィードバックを得られるようにしつつ、タグ push 時には macOS / Windows 向け成果物を GitHub Releases に自動公開できる状態を目指す。

## Goals

- `push` / `pull_request` で Rust コードの最低限の品質確認を自動化する
- `v*` タグ push をトリガーに macOS / Windows の成果物を GitHub Release へ自動公開する
- 配布物の命名と圧縮形式を OS ごとに分かりやすく統一する
- README に運用手順を追記し、メンテナが迷わずリリースできるようにする

## Non-Goals

- macOS / Windows のコード署名や notarization
- Linux 向け成果物の配布
- インストーラー生成
- 自動 version bump

## Approach

### 1. CI workflow

`.github/workflows/ci.yml` を追加し、`push` / `pull_request` を対象に以下を実行する。

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`

このリポジトリは Rust アプリケーションであり、README にも `cargo test` が既存の検証手段として記載されているため、まずは Rust 標準ツールチェーンだけで完結する CI を採用する。

### 2. Release workflow

`.github/workflows/release.yml` を追加し、`refs/tags/v*` をトリガーに matrix build を実行する。

- `macos-latest`
- `windows-latest`

各ジョブでは release build を生成し、OS ごとに成果物を圧縮する。

- macOS: `fee-backcalc-macos.tar.gz`
- Windows: `fee-backcalc-windows.zip`

その後、`softprops/action-gh-release` などの安定した GitHub Release 用 action を使って、同一タグの Release に成果物を添付する。

### 3. Artifact contents

配布物には最低限次を含める。

- 実行バイナリ
- 実行に必要な同梱ファイルが将来的に増えた場合でもまとめて配布できるディレクトリ構造

現状のリポジトリ構成上、アプリは単一バイナリでの配布が基本になるため、最初は実行ファイル中心のアーカイブで十分と判断する。

### 4. Release creation rules

- タグ名は `v0.1.0` のような形式を想定する
- タグ push 時に Release を自動作成または更新する
- Release 名はタグ名をそのまま利用する
- prerelease / draft は初期実装では扱わない

### 5. Documentation updates

README に以下を追記する。

- CI が実行する内容
- リリース方法
- `git tag vX.Y.Z && git push origin vX.Y.Z` で自動公開されること
- 生成される成果物の概要

## File Plan

- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Modify: `README.md`

必要に応じて GitHub Actions から参照するパスを安定させるため、将来的には配布用スクリプトを `scripts/` に切り出せるが、今回のスコープでは workflow 内の最小限の shell で十分。

## Data / Control Flow

### CI

1. 開発者が branch push または pull request を作成
2. GitHub Actions が Rust toolchain をセットアップ
3. `fmt`、`clippy`、`test` を順に実行
4. いずれかが失敗したら job fail

### Release

1. 開発者が `v*` タグを push
2. matrix build で macOS / Windows が並列ビルド
3. 各ジョブが release binary を生成
4. 各ジョブが OS 別アーカイブを作成
5. GitHub Release に成果物を添付

## Error Handling

- CI の各検証が失敗した場合はその時点でジョブを fail させる
- Release build が片方の OS で失敗した場合は workflow 全体を失敗として扱う
- タグ公開後に成果物不足の Release が残る事故を避けるため、各ジョブは成果物作成完了後にのみ upload を行う

## Testing Strategy

- workflow 追加後にローカルで `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- workflow YAML は構文ミスがないよう目視確認する
- 実リリース検証はタグ作成時の GitHub Actions 実行結果で確認する

## Open Decisions

- Windows で GUI 実行に必要な追加アセットが将来増えた場合は zip 内容を見直す
- macOS 向け署名が必要になった場合は別途 secrets と notarization 手順を追加する
