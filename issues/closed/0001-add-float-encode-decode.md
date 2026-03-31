# encode_f32/decode_f32 と encode_i24/decode_i24 を追加する

Created: 2026-03-22
Completed: 2026-03-22
Model: Opus 4.6

## 概要

`opus_encode_float` / `opus_decode_float` / `opus_encode24` / `opus_decode24` に対応する
Rust API をメソッド分離方式で追加する。

## 背景

現在 `Encoder::encode(&[i16])` と `Decoder::decode` (i16 出力) のみ提供している。

opus は非 FIXED_POINT ビルド (本クレートのデフォルト) では内部が float ベースで動作する。
i16 版を呼ぶと内部で i16→float 変換が発生するが、float 版なら変換なしで直通する。
ユーザー側で f32→i16 変換を行うと量子化ノイズが入るため、ライブラリ側で f32 API を提供すべき。

i24 (opus_encode24/opus_decode24) は opus v1.6.1 で追加された API で、
24bit ADC/DAC を使用するプロオーディオ環境で 24→16bit の切り捨てを避けるために使用する。
i32 の下位 24bit を使用する形式。

## 対応内容

### エンコーダー

- `Encoder::encode_f32(&mut self, pcm: &[f32]) -> Result<Vec<u8>, Error>` を追加する
  - 内部で `sys::opus_encode_float` を呼ぶ
- `Encoder::encode_i24(&mut self, pcm: &[i32]) -> Result<Vec<u8>, Error>` を追加する
  - 内部で `sys::opus_encode24` を呼ぶ
- PCM 長のバリデーションは `encode` と同じロジック

### デコーダー

- `Decoder::decode_f32(&mut self, encoded: &[u8]) -> Result<Vec<f32>, Error>` を追加する
- `Decoder::decode_fec_f32(&mut self, encoded: &[u8]) -> Result<Vec<f32>, Error>` を追加する
- `Decoder::decode_plc_f32(&mut self) -> Result<Vec<f32>, Error>` を追加する
  - 内部で `sys::opus_decode_float` を呼ぶ
- `Decoder::decode_i24(&mut self, encoded: &[u8]) -> Result<Vec<i32>, Error>` を追加する
- `Decoder::decode_fec_i24(&mut self, encoded: &[u8]) -> Result<Vec<i32>, Error>` を追加する
- `Decoder::decode_plc_i24(&mut self) -> Result<Vec<i32>, Error>` を追加する
  - 内部で `sys::opus_decode24` を呼ぶ

### テスト

- `encode_f32_silent` - 無音 f32 データのエンコード
- `encode_f32_decode_f32_roundtrip` - f32 でのラウンドトリップ
- `encode_f32_pcm_length_mismatch` - 長さ不一致エラー
- `decode_fec_f32` - FEC での f32 デコード
- `decode_plc_f32` - PLC での f32 デコード
- `encode_i24_silent` - 無音 i24 データのエンコード
- `encode_i24_decode_i24_roundtrip` - i24 でのラウンドトリップ
- `encode_i24_pcm_length_mismatch` - 長さ不一致エラー
- `decode_fec_i24` - FEC での i24 デコード
- `decode_plc_i24` - PLC での i24 デコード

## 解決方法

`Encoder` に `encode_f32()` / `encode_i24()` メソッドを追加し、
`Decoder` に `decode_f32()` / `decode_fec_f32()` / `decode_plc_f32()` /
`decode_i24()` / `decode_fec_i24()` / `decode_plc_i24()` メソッドを追加した。

エンコーダー側の PCM 長バリデーションは `check_pcm_length()` に共通化した。
