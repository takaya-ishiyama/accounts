# リポジトリガイドライン

## プロジェクト構成とモジュール編成

これは Rust 2021 の Cloudflare Workers 認証サービスです。Worker のエントリポイントは `src/lib.rs` で、Axum ルート、キュー処理、セキュリティヘッダーを接続しています。

- `src/adapter/`: HTTP API ハンドラーと境界コード。
- `src/application/`: ユースケース、コマンド/クエリフロー、プロジェクション、HTTP ヘルパー。
- `src/domain/`: モデル、認証ルール、イベント、純粋なビジネスロジック。
- `src/infrastructure/`: Cloudflare バインディング、リポジトリ、プロバイダー、トークン、セッション、環境アダプター。
- `db/schema.sql` と `db/migrations/`: Atlas 管理の D1 スキーマとマイグレーション。
- `docs/`、`README.md`、`USAGE.md`: 設計メモと運用上の使い方。

## ビルド、テスト、開発コマンド

- `mise run dev`: Wrangler 開発サーバーをポート `8000` で起動します。
- `cargo test`: Rust のユニットテストを実行します。
- `cargo fmt --check`: Rust のフォーマットを確認します。
- `cargo clippy --all-targets --all-features`: Rust の lint を実行します。
- `mise run migrate-local`: 初期 D1 マイグレーションをローカルに適用します。
- `mise run migrate-diff`: `db/schema.sql` から Atlas のマイグレーション差分を生成します。
- `MISE_ENV=staging mise run migrate-staging` / `mise run deploy-staging`: staging へ適用およびデプロイします。
- `MISE_ENV=production mise run migrate-production` / `mise run deploy-production`: production へ適用およびデプロイします。

Wrangler のビルド設定は `wrangler.toml` にあります。`worker-build@^0.8` をインストールし、release WASM をビルドします。

## コーディングスタイルと命名規則

`rustfmt` による標準的な Rust フォーマットを使い、import を整理し、モジュール境界を明確に保ってください。`refresh_token_store.rs` や `auth_google_callback.rs` のような既存の snake_case ファイル名に従ってください。ドメイン型は `PascalCase`、関数と変数は `snake_case`、定数は必要に応じて `SCREAMING_SNAKE_CASE` を使います。ドメインコードから Cloudflare API を切り離し、Worker、D1、KV、キュー、プロバイダー連携は `infrastructure` または `adapter` に配置してください。

## テストガイドライン

ユニットテストは実装ファイルの近くにある `#[cfg(test)] mod tests` ブロックに置きます。振る舞いが変わる場合は、ドメインルール、パーサー、バリデーション、ユースケース分岐のテストを追加してください。ネットワーク呼び出しよりも、ローカルフィクスチャを使った決定的なテストを優先します。変更を提出する前に `cargo test` を実行してください。Worker バインディングに統合カバレッジが必要な場合を除き、変更したモジュールの近くに焦点を絞ったテストを追加します。

## コミットとプルリクエストのガイドライン

直近の履歴では `fix state`、`refactor`、`usage` のような短い命令形の件名が使われています。コミットは小さく、焦点を絞ってください。プルリクエストには概要、テスト結果（`cargo test`、フォーマット/lint チェック）、`db/` 変更のマイグレーションメモ、`wrangler.toml` や環境の振る舞いが変わる場合のデプロイまたはシークレット更新情報を含めてください。

## セキュリティと設定の注意点

`.env.local`、`.env.staging`、`.env.production`、または Wrangler のシークレット値をコミットしないでください。環境固有の振る舞いを変更する場合は、`wrangler.toml` の OAuth リダイレクト URI、JWT 設定、D1 データベース名、KV namespace ID、Durable Objects、キューバインディングを確認してください。
