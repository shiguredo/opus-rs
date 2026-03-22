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

### 新しい型

- `DredDecoder` - DRED デコーダー (`OpusDREDDecoder` のラッパー)
- `Dred` - DRED 状態 (`OpusDRED` のラッパー)

### DredDecoder のメソッド

- `DredDecoder::new() -> Result<Self, Error>` - DRED デコーダーを作成する
- `DredDecoder::parse(&mut self, dred: &mut Dred, data: &[u8], max_dred_samples: i32, sampling_rate: i32) -> Result<i32, Error>` - DRED パケットをパースする
- `DredDecoder::process(&mut self, src: &Dred, dst: &mut Dred) -> Result<(), Error>` - 遅延処理を完了する

### Dred のメソッド

- `Dred::new() -> Result<Self, Error>` - DRED 状態を作成する

### Decoder への追加メソッド

- `Decoder::dred_decode(&mut self, dred: &Dred, dred_offset: i32) -> Result<Vec<i16>, Error>` - DRED からオーディオをデコードする
- `Decoder::dred_decode_f32(&mut self, dred: &Dred, dred_offset: i32) -> Result<Vec<f32>, Error>` - DRED からオーディオを f32 デコードする
- `Decoder::dred_decode_i24(&mut self, dred: &Dred, dred_offset: i32) -> Result<Vec<i32>, Error>` - DRED からオーディオを i24 デコードする

### エンコーダー CTL

- `EncoderConfig::dred_duration` - DRED の最大フレーム数を設定する (10ms 単位)

### feature フラグ

- `dred` feature で有効化する (`source-build` を暗黙的に要求)

### テスト

- DRED エンコード→パース→デコードのラウンドトリップテスト (サイン波)
- DRED が無効な場合のパーステスト

## pending にした理由

Rust バインディング側の実装は完了しているが、opus ライブラリのビルドに問題がある。

DRED は DNN (Deep Neural Network) コンポーネントを必要とし、
cmake の `OPUS_DRED=ON` オプションで有効にする。しかし、DNN の学習済みデータヘッダー
(`dnn/fargan_data.h` 等) は `autogen.sh` や専用スクリプトで生成する必要があり、
git clone + cmake だけではビルドできない。

opus v1.6.1 のリリースソースアーカイブ (tar.gz) にはこれらのファイルが含まれるが、
現在の build.rs は git clone でソースを取得しているため、生成済みファイルが存在しない。

### 解決に必要な対応

- ソース取得方法をリリースアーカイブに変更する、または
- `autogen.sh` をビルドプロセスに組み込む、または
- 生成済みデータファイルを別途取得する仕組みを用意する
