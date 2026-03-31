# パケットパースと LBRR 検出を追加する

Created: 2026-03-22
Model: Opus 4.6

## 概要

`opus_packet_parse` と `opus_packet_has_lbrr` の Rust API を追加する。

## 背景

issue 0002 でパケットユーティリティ関数の大部分を追加したが、
`opus_packet_parse` (パケットの内部フレーム構造の解析) と
`opus_packet_has_lbrr` (LBRR 冗長データの有無判定) が未実装。

`opus_packet_parse` はパケット内の個別フレームへのアクセスが必要な
高度なストリーム処理で使用される。

## 対応内容

### モジュール関数

- `packet_parse(data: &[u8]) -> Result<PacketInfo, Error>` - パケットを解析してフレーム情報を返す
- `packet_has_lbrr(packet: &[u8]) -> Result<bool, Error>` - パケットに LBRR データが含まれるか判定する

### 新しい型

- `PacketInfo` - パース結果 (TOC バイト、フレームデータへの参照、フレームサイズ、ペイロードオフセット)

### テスト

- エンコードしたパケットの parse テスト
- LBRR の有無判定テスト
