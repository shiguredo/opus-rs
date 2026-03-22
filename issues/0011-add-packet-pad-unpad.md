# パケットパディング操作を追加する

Created: 2026-03-22
Model: Opus 4.6

## 概要

`opus_packet_pad` / `opus_packet_unpad` の Rust API を追加する。

## 背景

パケットパディングは CBR (固定ビットレート) 伝送で
全パケットを同じサイズに揃える必要がある場合に使用される。
`opus_packet_pad` でパケットを指定サイズにパディングし、
`opus_packet_unpad` でパディングを除去する。

## 対応内容

### モジュール関数

- `packet_pad(data: &mut Vec<u8>, new_len: usize) -> Result<(), Error>` - パケットを指定サイズにパディングする
- `packet_unpad(data: &mut Vec<u8>) -> Result<usize, Error>` - パケットからパディングを除去する

### テスト

- パディング→アンパディングのラウンドトリップテスト
- パディングされたパケットが正常にデコードできることの確認
