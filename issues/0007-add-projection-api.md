# Projection/Ambisonics API を追加する

Created: 2026-03-22
Model: Opus 4.6

## 概要

opus の Projection/Ambisonics API の Rust バインディングを追加する。

## 背景

Projection API は Ambisonics 形式のマルチチャンネル空間オーディオ処理を行う。
opus v1.2 から stable として提供されている。

通常の `Encoder`/`Decoder` とは完全に別の型
(`OpusProjectionEncoder`/`OpusProjectionDecoder`) で構成され、
demixing matrix やストリーム/カップルドストリームの管理が必要。

## 対応内容

### 新しい型

- `ProjectionEncoder` - Projection エンコーダー (`OpusProjectionEncoder` のラッパー)
- `ProjectionDecoder` - Projection デコーダー (`OpusProjectionDecoder` のラッパー)
- `ProjectionEncoderConfig` - エンコーダー設定
- `ProjectionDecoderConfig` - デコーダー設定

### ProjectionEncoder のメソッド

- `ProjectionEncoder::new(config: ProjectionEncoderConfig) -> Result<Self, Error>`
- `ProjectionEncoder::encode(&mut self, pcm: &[i16]) -> Result<Vec<u8>, Error>`
- `ProjectionEncoder::encode_f32(&mut self, pcm: &[f32]) -> Result<Vec<u8>, Error>`
- `ProjectionEncoder::encode_i24(&mut self, pcm: &[i32]) -> Result<Vec<u8>, Error>`
- `ProjectionEncoder::get_demixing_matrix(&self) -> Result<Vec<u8>, Error>` - demixing matrix を取得する
- `ProjectionEncoder::get_demixing_matrix_gain(&self) -> Result<i32, Error>` - demixing matrix のゲインを取得する

### ProjectionDecoder のメソッド

- `ProjectionDecoder::new(config: ProjectionDecoderConfig) -> Result<Self, Error>`
- `ProjectionDecoder::decode(&mut self, data: &[u8]) -> Result<Vec<i16>, Error>`
- `ProjectionDecoder::decode_f32(&mut self, data: &[u8]) -> Result<Vec<f32>, Error>`
- `ProjectionDecoder::decode_i24(&mut self, data: &[u8]) -> Result<Vec<i32>, Error>`

### テスト

- Ambisonics エンコード→デコードのラウンドトリップテスト (サイン波)
- demixing matrix の取得テスト
