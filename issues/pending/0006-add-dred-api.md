# DRED (Deep Redundancy) API を追加する

Created: 2026-03-22
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

### Rust 側の実装 (完了済み)

- `DredDecoder` / `Dred` 型
- `Decoder::dred_decode()` / `dred_decode_f32()` / `dred_decode_i24()`
- `EncoderConfig::dred_duration` / `Encoder::get_dred_duration()`
- 全コードは `#[cfg(feature = "dred")]` でゲート済み
- `dred` feature は `source-build` を暗黙的に要求

### テスト (実装済み、未実行)

- DRED エンコード→パース→デコードのラウンドトリップテスト (サイン波)
- DRED が無効な場合のパーステスト

## pending にした理由

opus の DRED ビルドに必要な DNN 学習済みモデルデータファイルが取得できない。

DRED は cmake の `OPUS_DRED=ON` で有効になるが、DNN コンポーネントが
以下の学習済みデータヘッダーを必要とする:

- `dnn/fargan_data.h` / `dnn/fargan_data.c`
- `dnn/plc_data.h` / `dnn/plc_data.c`
- `dnn/pitchdnn_data.h` / `dnn/pitchdnn_data.c`
- `dnn/dred_rdovae_enc_data.h` / `dnn/dred_rdovae_dec_data.h`
- 他 6 ファイル

これらのファイルは Python スクリプト (`dump_fargan_weights.py` 等) で
学習済みモデルから生成するもので、**git リポジトリには含まれていない**。

一方、**リリースアーカイブ** (`opus-1.6.1.tar.gz`) には全て含まれている。

### 現状

build.rs は `git clone https://github.com/xiph/opus.git --branch v1.6.1` で
ソースを取得しているため、生成済みデータファイルが存在せずビルドが失敗する。

### 解決に必要な対応

build.rs の source-build 時のソース取得方法を
git clone からリリースアーカイブ (`https://downloads.xiph.org/releases/opus/opus-1.6.1.tar.gz`)
のダウンロード・展開に変更する。
