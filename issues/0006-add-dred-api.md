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

### テスト

- DRED エンコード→パース→デコードのラウンドトリップテスト (サイン波)
- DRED が無効な場合のパーステスト
