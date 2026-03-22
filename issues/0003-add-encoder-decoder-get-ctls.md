# エンコーダー/デコーダーの GET 系 CTL を追加する

Created: 2026-03-22
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

- `get_bitrate() -> Result<u32, Error>` - 現在のビットレートを取得する
- `get_bandwidth() -> Result<Bandwidth, Error>` - 現在の帯域幅を取得する
- `get_complexity() -> Result<u8, Error>` - 現在の計算量を取得する
- `get_vbr() -> Result<bool, Error>` - VBR 設定を取得する
- `get_inband_fec() -> Result<InbandFec, Error>` - FEC 設定を取得する
- `get_dtx() -> Result<bool, Error>` - DTX 設定を取得する
- `get_sample_rate() -> Result<u32, Error>` - サンプルレートを取得する

### デコーダー GET 系

- `get_bandwidth() -> Result<Bandwidth, Error>` - 最後にデコードしたパケットの帯域幅を取得する
- `get_gain() -> Result<i32, Error>` - 現在のゲイン設定を取得する
- `get_last_packet_duration() -> Result<usize, Error>` - 最後にデコードしたパケットのサンプル数を取得する
- `get_pitch() -> Result<i32, Error>` - 最後にデコードしたフレームのピッチ周期を取得する

### テスト

- エンコーダー: 設定した値が GET で取得できることの確認
- デコーダー: デコード後の状態取得の確認
- デフォルト値の確認
