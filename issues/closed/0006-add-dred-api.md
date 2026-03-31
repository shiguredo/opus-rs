# DRED (Deep Redundancy) API を追加する

Created: 2026-03-22
Completed: 2026-03-22
Model: Opus 4.6

## 概要

opus v1.5 で追加された DRED (Deep Redundancy) API の Rust バインディングを追加する。

## 背景

DRED は Deep Learning ベースの冗長性機能で、パケットロスが多い環境での
オーディオ品質を大幅に改善する。従来の FEC (前方誤り訂正) よりも
長期間のパケットロスに対応可能。

WebRTC のパケットロス対策として実用的であり、
既存の `Encoder` / `Decoder` と組み合わせて使用する。

## 対応内容

### 新しい型

- `DredDecoder` - DRED デコーダー (`OpusDREDDecoder` のラッパー)
- `Dred` - DRED 状態 (`OpusDRED` のラッパー)

### DredDecoder のメソッド

- `DredDecoder::new() -> Result<Self, Error>`
- `DredDecoder::parse()` - DRED パケットをパースする
- `DredDecoder::process()` - 遅延処理を完了する

### Decoder への追加メソッド

- `Decoder::dred_decode()` / `dred_decode_f32()` / `dred_decode_i24()`

### エンコーダー CTL

- `EncoderConfig::dred_duration` / `Encoder::get_dred_duration()`

### feature フラグ

- `dred` feature で有効化 (`source-build` を暗黙的に要求)
- 全コードは `#[cfg(feature = "dred")]` でゲート

## 解決方法

Rust 側の実装は `DredDecoder` / `Dred` 型と `Decoder::dred_decode*` メソッドを追加した。

ビルド問題は、ソース取得方法を git clone からリリースアーカイブのダウンロードに変更して解決した。
git リポジトリには DNN 学習済みデータファイル (`*_data.h` / `*_data.c`) が含まれないが、
リリースアーカイブには含まれている。

Cargo.toml の metadata を `url` / `mirror_url` / `version` / `sha256` 形式に変更し、
build.rs で mirror_url を優先してダウンロード、失敗時に url にフォールバックする設計にした。
