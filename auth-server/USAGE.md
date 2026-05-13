# Auth Server Usage

この auth-server は、複数アプリケーションから使う中央認証基盤です。

各アプリケーションは auth-server の Cookie を自分のログインセッションとして使いません。auth-server は短命の app authorization code をアプリの backend に返し、アプリ backend が `/oauth/token` で access token / refresh token に交換します。

## 登場するセッションと token

- app session: 各アプリケーションが自ドメインで発行するログインセッション
- app authorization code: auth-server からアプリ callback に返す短命 one-time code
- access token: 各アプリ API が検証する RS256 JWT
- refresh token: access token 更新用の opaque token

## Auth Server 側の設定

`wrangler.toml` の環境ごとに `APP_CLIENTS` を設定します。

```toml
APP_CLIENTS='[
	  {
	    "client_id": "cook-note-web",
	    "client_secret": "replace-with-secret",
	    "token_audience": "cook-note",
	    "redirect_uri": "https://cook-note.example.com/auth/callback"
	  }
]'
```

各項目の意味:

- `client_id`: アプリケーション識別子
- `client_secret`: アプリ backend が `/oauth/token` で使う secret。ブラウザには渡さない
- `token_audience`: 発行する access token の `aud`
- `redirect_uri`: app authorization code を返すアプリ側 callback URL

アプリ backend 側には少なくとも次の環境変数を置く想定です。

| 名前 | 用途 |
|---|---|
| `AUTH_SERVER_URL` | auth-server の origin。例: `https://auth.example.com` |
| `AUTH_CLIENT_ID` | `APP_CLIENTS[].client_id` と同じ値 |
| `AUTH_CLIENT_SECRET` | `APP_CLIENTS[].client_secret` と同じ値。backend の secret として管理する |

主な環境変数:

| 名前 | 用途 |
|---|---|
| `GOOGLE_CLIENT_ID` | Google OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | Google OAuth client secret |
| `OAUTH_REDIRECT_URI` | Google から auth-server へ戻る callback URL |
| `JWT_ISSUER` | access token の `iss` |
| `JWT_PRIVATE_KEY` | JWT 署名用 RSA private key |
| `JWT_PREVIOUS_PRIVATE_KEY` | 直前の JWT 署名鍵。ローテーション用 |
| `JWT_DEFAULT_AUDIENCE` | 既存互換用の default audience |
| `ALLOWED_AUDIENCES` | 検証で許可する audience |
| `APP_CLIENTS` | 登録済みアプリ設定 JSON |
| `SESSION_TTL_SECS` | access token / auth-server session JWT の TTL。未設定時 3600 秒 |
| `APP_AUTH_CODE_TTL_SECS` | app authorization code の TTL。未設定時 300 秒 |
| `REFRESH_TOKEN_TTL_SECS` | refresh token の TTL。未設定時 30 日 |

## アプリケーション側ログインフロー

### 1. アプリの `/login` から auth-server へ redirect

アプリ backend はブラウザを auth-server に redirect します。

```http
GET https://auth.example.com/auth/google/start?client_id=cook-note-web&return_to=%2Fdashboard&state=<app_state>
```

`return_to` は相対パスだけを渡してください。

`state` はアプリ backend がログイン開始ごとに生成するランダム値です。アプリ backend はこの値を自アプリ側の一時 session などに保存し、callback で戻ってきた `state` と一致することを確認してください。

有効な例:

```text
/
/dashboard
/notes/123?tab=edit
```

無効な例:

```text
https://evil.example.com
//evil.example.com
/path\to
```

### 2. Google ログイン後、auth-server がアプリ callback へ戻す

auth-server は `client_id` に登録された `redirect_uri` にだけ redirect します。

```http
GET https://cook-note.example.com/auth/callback?code=...&return_to=%2Fdashboard&state=<app_state>
```

この `code` は Google OAuth code ではなく、auth-server が発行した app authorization code です。
アプリ backend は `state` がログイン開始時に保存した値と一致することを確認してから、`code` を token に交換してください。

### 3. アプリ backend が code を token に交換

アプリ backend から auth-server に server-to-server で送ります。

```http
POST https://auth.example.com/oauth/token
Content-Type: application/json
Authorization: Basic base64(AUTH_CLIENT_ID:AUTH_CLIENT_SECRET)

{
  "grant_type": "authorization_code",
  "code": "..."
}
```

成功レスポンス:

```http
Cache-Control: no-store
Pragma: no-cache
```

```json
{
  "access_token": "...",
  "refresh_token": "...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

アプリ backend はこの後、自アプリの session cookie を発行します。

```http
Set-Cookie: app-session=...; Path=/; Secure; HttpOnly; SameSite=Lax
```

その後、`return_to` へ redirect します。

## Access Token の使い方

アプリ API にリクエストする場合は、access token を Bearer token として送ります。

```http
GET https://api.cook-note.example.com/me
Authorization: Bearer <access_token>
```

アプリ API は auth-server の JWKS を取得し、JWT を検証します。

```http
GET https://auth.example.com/.well-known/jwks.json
```

検証する claim:

- `iss`: `JWT_ISSUER` と一致すること
- `aud`: アプリの `token_audience` と一致すること
- `exp`: 期限切れでないこと
- 署名: JWKS の公開鍵で検証できること

主な claim:

```json
{
  "iss": "https://auth.example.com",
  "sub": "01JGVQ...",
  "aud": "cook-note",
  "iat": 1745000000,
  "exp": 1745003600,
  "jti": "01JGVS...",
  "account_version": 1
}
```

## Refresh Token の使い方

access token が期限切れになる前後に、アプリ backend から refresh します。

```http
POST https://auth.example.com/oauth/token
Content-Type: application/json
Authorization: Basic base64(AUTH_CLIENT_ID:AUTH_CLIENT_SECRET)

{
  "grant_type": "refresh_token",
  "refresh_token": "..."
}
```

成功すると、新しい access token と refresh token が返ります。

```http
Cache-Control: no-store
Pragma: no-cache
```

```json
{
  "access_token": "...",
  "refresh_token": "...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

refresh token はローテーションされます。成功後は古い refresh token を破棄し、必ず新しい refresh token を保存してください。

refresh token はアプリ backend 側で安全に保存してください。ブラウザ JavaScript から読める場所には置かないでください。

## Logout

現時点では refresh token revoke API はありません。

アプリ側 logout では、アプリ自身の session cookie を削除します。

```http
Set-Cookie: app-session=; Max-Age=0; Path=/; Secure; HttpOnly; SameSite=Lax
```

注意点:

- auth-server 側の refresh token は TTL まで自然失効します
- 漏洩した refresh token を即時 revoke する API はありません
- refresh token grant 時に account が停止済みの場合は token を発行しません

## Auth Server 自身の API

### Health check

```http
GET /healthz
```

### JWKS

```http
GET /.well-known/jwks.json
```

### OpenID configuration

```http
GET /.well-known/openid-configuration
```

## ローカル開発

```sh
mise run dev
```

staging / production migration:

```sh
MISE_ENV=staging mise run migrate-staging
MISE_ENV=production mise run migrate-production
```

## 典型的なアプリ backend 実装

1. `/login` で auth-server の `/auth/google/start` に redirect
2. ログイン開始時にアプリ側 `state` を生成・保存し、`/auth/google/start` に渡す
3. `/auth/callback` で `code`, `return_to`, `state` を受け取る
4. callback の `state` が保存済みの値と一致することを確認する
5. backend から `/oauth/token` に authorization code exchange を送る
6. 返ってきた token を backend 側 session に保存する
7. app session cookie を発行する
8. `return_to` に redirect する
9. access token 期限切れ時は refresh token grant で更新する

## 制限事項

- public client / frontend-only token exchange は対象外
- auth-server の current account API は未実装
- refresh token revoke API は未実装
- device 一覧、session 一覧は未実装
- consent 画面は未実装
- 1 client 1 redirect URI のみ対応
