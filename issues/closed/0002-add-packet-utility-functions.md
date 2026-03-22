# パケットユーティリティ関数を追加する

Created: 2026-03-22
Completed: 2026-03-22
Model: Opus 4.6

## 概要

opus のパケット解析・操作関数群の Rust API を追加する。

## 背景

Opus パケットのメタデータ取得はストリーム処理で不可欠。
現在パケットの帯域幅、チャンネル数、フレーム数などを取得する手段がなく、
ユーザーが Opus パケットを適切に処理できない。

これらは Decoder インスタンスを必要としないスタンドアロン関数であり、
パケットルーティングやストリーム解析に使用される。

## 対応内容

### パケット情報取得関数 (モジュール関数として追加)

- `packet_get_bandwidth(packet: &[u8]) -> Result<Bandwidth, Error>` - パケットの帯域幅を取得する
- `packet_get_nb_channels(packet: &[u8]) -> Result<u8, Error>` - パケットのチャンネル数を取得する
- `packet_get_nb_frames(packet: &[u8]) -> Result<usize, Error>` - パケットに含まれるフレーム数を取得する
- `packet_get_samples_per_frame(packet: &[u8], sample_rate: u32) -> Result<usize, Error>` - 1 フレームあたりのサンプル数を取得する
- `packet_get_nb_samples(packet: &[u8], sample_rate: u32) -> Result<usize, Error>` - パケット全体のサンプル数を取得する

### Bandwidth の from_opus 変換

- `Bandwidth` に `from_opus(value: i32) -> Option<Bandwidth>` を追加する (C 定数からの逆変換)

### テスト

- 各関数に対して、実際にエンコードしたパケットを使ったテスト
- 不正なパケット (空、切り詰め) に対するエラーハンドリングテスト
- 各サンプルレートでの samples_per_frame のテスト

## 解決方法

5 つのパケットユーティリティ関数をモジュール関数として追加した。
`Bandwidth::from_opus()` で C 定数からの逆変換を実装した。
空パケットに対するバリデーションを Rust 側で行い、null ポインタを C API に渡さないようにした。
