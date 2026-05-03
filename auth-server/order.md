## 概要
アカウント（認証）管理のためのアカウント基盤を作成したい。その設計をお願いする。以下が要求である

## 詳細要求
* 設計をmarkdown形式のファイルに出力してほしい
* 認証は拡張する予定だが、はじめはgoogleでログイン（OIDCとOAuthを想定）のみでよい
* cloudflare workerにrustをwasmにビルドしてデプロイする想定。参考は下記
https://developers.cloudflare.com/workers/languages/rust/
* イベントソーシングで行う
* コマンドはdurable objects、queryはD1を採用し、cloudflare queueで同期させる
* dbマイグレーションはatlasを用いる
