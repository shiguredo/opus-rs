# Repacketizer を追加する

Created: 2026-03-22
Model: Opus 4.6

## 概要

`opus_repacketizer_*` 関数群の Rust API を追加する。

## 背景

Repacketizer はサーバーサイドでのパケット操作に使用される。
複数の Opus パケットを結合したり、パケット内の特定フレームだけを
取り出して新しいパケットを構成する機能を提供する。

SFU (Selective Forwarding Unit) やメディアサーバーでの
パケット再構成に不可欠。

## 対応内容

### 新しい型

- `Repacketizer` - パケット再構成器 (`OpusRepacketizer` のラッパー)

### メソッド

- `Repacketizer::new() -> Result<Self, Error>` - 作成する
- `Repacketizer::reset(&mut self)` - 状態をリセットする
- `Repacketizer::cat(&mut self, data: &[u8]) -> Result<(), Error>` - パケットを追加する
- `Repacketizer::out(&mut self, max_len: usize) -> Result<Vec<u8>, Error>` - 結合パケットを出力する
- `Repacketizer::out_range(&mut self, begin: usize, end: usize, max_len: usize) -> Result<Vec<u8>, Error>` - 指定範囲のフレームを出力する
- `Repacketizer::get_nb_frames(&self) -> usize` - 追加済みフレーム数を取得する

### テスト

- 複数パケットの結合テスト
- フレーム範囲指定の出力テスト
