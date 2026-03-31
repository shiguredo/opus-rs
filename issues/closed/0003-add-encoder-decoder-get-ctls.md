# エンコーダー/デコーダーの GET 系 CTL を追加する

Created: 2026-03-22
Completed: 2026-03-22
Model: Opus 4.6

## 概要

エンコーダーとデコーダーの状態を取得する GET 系 CTL メソッドを追加する。

## 背景

現在 SET 系 CTL は `EncoderConfig` で網羅的に設定できるが、
GET 系は `get_lookahead` のみ。設定後の値の確認や、
エンコーダーが自動選択した帯域幅・モードの取得手段がない。

デバッグやアダプティブビットレート制御で現在の状態を取得する必要がある。

## 対応内容

### エンコーダー GET 系

- `get_bitrate() -> Result<u32, Error>`
- `get_bandwidth() -> Result<Bandwidth, Error>`
- `get_complexity() -> Result<u8, Error>`
- `get_vbr() -> Result<bool, Error>`
- `get_inband_fec() -> Result<InbandFec, Error>`
- `get_dtx() -> Result<bool, Error>`
- `get_sample_rate() -> Result<u32, Error>`

### デコーダー GET 系

- `get_bandwidth() -> Result<Bandwidth, Error>`
- `get_gain() -> Result<i32, Error>`
- `get_last_packet_duration() -> Result<usize, Error>`
- `get_pitch() -> Result<i32, Error>`

### テスト

- エンコーダー: 設定した値が GET で取得できることの確認
- デコーダー: デコード後の状態取得の確認
- デフォルト値の確認

## 解決方法

エンコーダーに 7 個、デコーダーに 4 個の GET 系 CTL メソッドを追加した。
全メソッドで `opus_encoder_ctl` / `opus_decoder_ctl` を呼び出し、
エラーチェックと型変換を行う。
