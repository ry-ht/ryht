# Claude Code SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/cc-sdk.svg)](https://crates.io/crates/cc-sdk)
[![Documentation](https://docs.rs/cc-sdk/badge.svg)](https://docs.rs/cc-sdk)
[![License](https://img.shields.io/crates/l/cc-sdk.svg)](LICENSE)

Claude Code CLIと対話するためのRust SDKです。シンプルなクエリインターフェースと完全なインタラクティブクライアント機能を提供し、Python SDKとほぼ同等の機能を実現しています。

## 機能

- 🚀 **シンプルクエリインターフェース** - `query()` 関数による一度きりのクエリ
- 💬 **インタラクティブクライアント** - コンテキストを保持したステートフルな会話
- 🔄 **ストリーミングサポート** - リアルタイムメッセージストリーミング
- 🛑 **中断機能** - 実行中の操作をキャンセル
- 🔧 **完全な設定** - Python SDKと同等の包括的なオプション
- 📦 **型安全性** - serdeによる強い型付けサポート
- ⚡ **非同期/待機** - Tokioベースの非同期操作

## インストール

`Cargo.toml` に以下を追加：

```toml
[dependencies]
cc-sdk = "0.2.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

## 前提条件

Claude Code CLIをインストール：

```bash
npm install -g @anthropic-ai/claude-code
```

## クイックスタート

### シンプルクエリ（一度きり）

```rust
use cc_sdk::{query, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut messages = query("2 + 2はいくつですか？", None).await?;

    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }

    Ok(())
}
```

### インタラクティブクライアント

```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // メッセージを送信してレスポンスを受信
    let messages = client.send_and_receive(
        "Pythonのウェブサーバーを書いてください".to_string()
    ).await?;

    // レスポンスを処理
    for msg in &messages {
        match msg {
            cc_sdk::Message::Assistant { message } => {
                println!("Claude: {:?}", message);
            }
            _ => {}
        }
    }

    // フォローアップを送信
    let messages = client.send_and_receive(
        "async/awaitを使うようにしてください".to_string()
    ).await?;

    client.disconnect().await?;
    Ok(())
}
```

### 高度な使用方法

```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // レスポンスを待たずにメッセージを送信
    client.send_message("円周率を100桁まで計算してください".to_string()).await?;

    // 他の作業を実行...

    // 準備ができたらレスポンスを受信（Resultメッセージで停止）
    let messages = client.receive_response().await?;

    // 長時間実行される操作をキャンセル
    client.send_message("10000語のエッセイを書いてください".to_string()).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    client.interrupt().await?;

    client.disconnect().await?;
    Ok(())
}
```

## 設定オプション

```rust
use cc_sdk::{ClaudeCodeOptions, PermissionMode};

let options = ClaudeCodeOptions::builder()
    .system_prompt("あなたは役立つコーディングアシスタントです")
    .model("claude-3-5-sonnet-20241022")
    .permission_mode(PermissionMode::AcceptEdits)
    .max_turns(10)
    .max_thinking_tokens(10000)
    .allowed_tools(vec!["read_file".to_string(), "write_file".to_string()])
    .cwd("/path/to/project")
    .build();
```

### コントロールプロトコル（v0.1.12+）

Python Agent SDK と整合する新しいランタイム制御とオプション：

- `Query::set_permission_mode("acceptEdits" | "default" | "plan" | "bypassPermissions")`
- `Query::set_model(Some("sonnet"))` または `set_model(None)` で解除
- `ClaudeCodeOptions::builder().include_partial_messages(true)` で部分チャンクを有効化
- `Query::stream_input(stream)` は送信完了後に `end_input()` を自動呼び出し

### Agent ツール & MCP

- ツールの許可/禁止リスト：`ClaudeCodeOptions` の `allowed_tools` / `disallowed_tools`
- 権限モード：`PermissionMode::{Default, AcceptEdits, Plan, BypassPermissions}`
- 実行時承認：`CanUseTool` を実装して `PermissionResult::{Allow,Deny}` を返す
- MCP サーバー：`options.mcp_servers` で構成（stdio/http/sse/sdk）。SDK は `--mcp-config` に打包

## API リファレンス

### `query()`

一度きりの対話のためのシンプルでステートレスなクエリ関数。

```rust
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeCodeOptions>
) -> Result<impl Stream<Item = Result<Message>>>
```

### `InteractiveClient`

ステートフルでインタラクティブな会話のためのメインクライアント。

#### メソッド

- `new(options: ClaudeCodeOptions) -> Result<Self>` - 新しいクライアントを作成
- `connect() -> Result<()>` - Claude CLIに接続
- `send_and_receive(prompt: String) -> Result<Vec<Message>>` - メッセージを送信して完全なレスポンスを待つ
- `send_message(prompt: String) -> Result<()>` - 待機せずにメッセージを送信
- `receive_response() -> Result<Vec<Message>>` - Resultメッセージまでメッセージを受信
- `interrupt() -> Result<()>` - 実行中の操作をキャンセル
- `disconnect() -> Result<()>` - Claude CLIから切断

## メッセージタイプ

- `UserMessage` - ユーザー入力メッセージ
- `AssistantMessage` - Claudeのレスポンス
- `SystemMessage` - システム通知
- `ResultMessage` - タイミングとコスト情報を含む操作結果

## エラーハンドリング

SDKは包括的なエラー型を提供：

- `CLINotFoundError` - Claude Code CLIがインストールされていない
- `CLIConnectionError` - 接続エラー
- `ProcessError` - CLIプロセスエラー
- `InvalidState` - 無効な操作状態

## 例

使用例については `examples/` ディレクトリを参照：

- `interactive_demo.rs` - インタラクティブ会話デモ
- `query_simple.rs` - シンプルクエリ例
- `file_operations.rs` - ファイル操作例

## ライセンス

このプロジェクトはMITライセンスの下でライセンスされています - 詳細は [LICENSE](LICENSE) ファイルを参照してください。

## 貢献

貢献を歓迎します！お気軽にPull Requestを提出してください。
