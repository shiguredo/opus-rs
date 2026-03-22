# 変更履歴

- UPDATE
  - 後方互換がある変更
- ADD
  - 後方互換がある追加
- CHANGE
  - 後方互換のない変更
- FIX
  - バグ修正

## develop

- [UPDATE] Opus ライブラリのバージョンを v1.5.2 から v1.6.1 に更新する
  - @voluntas
- [UPDATE] エラーメッセージにパッケージ名を含めるようにする
  - @voluntas
- [ADD] DRED (Deep Redundancy) API を `dred` feature として追加する (`DredDecoder` / `Dred` / `Decoder::dred_decode()`)
  - @voluntas
- [ADD] エンコーダーに GET 系 CTL `get_bitrate()` / `get_bandwidth()` / `get_complexity()` / `get_vbr()` / `get_inband_fec()` / `get_dtx()` / `get_sample_rate()` を追加する
  - @voluntas
- [ADD] デコーダーに GET 系 CTL `get_bandwidth()` / `get_gain()` / `get_last_packet_duration()` / `get_pitch()` を追加する
  - @voluntas
- [ADD] パケットユーティリティ関数 `packet_get_bandwidth()` / `packet_get_nb_channels()` / `packet_get_nb_frames()` / `packet_get_samples_per_frame()` / `packet_get_nb_samples()` を追加する
  - @voluntas
- [ADD] `Bandwidth::from_opus()` による C 定数からの逆変換を追加する
  - @voluntas
- [ADD] エンコーダーに f32 入力の `Encoder::encode_f32()` を追加する
  - @voluntas
- [ADD] エンコーダーに 24bit 入力の `Encoder::encode_i24()` を追加する
  - @voluntas
- [ADD] デコーダーに f32 出力の `Decoder::decode_f32()` / `decode_fec_f32()` / `decode_plc_f32()` を追加する
  - @voluntas
- [ADD] デコーダーに 24bit 出力の `Decoder::decode_i24()` / `decode_fec_i24()` / `decode_plc_i24()` を追加する
  - @voluntas
- [ADD] prebuilt バイナリダウンロードに対応する
  - @voluntas
- [ADD] 静的ライブラリのシンボル名を `shiguredo_opus_` プレフィックス付きに書き換える機能を追加する
  - @voluntas
- [ADD] `EncoderConfig` による詳細なエンコーダー設定に対応する
  - @voluntas
- [ADD] `DecoderConfig` による詳細なデコーダー設定に対応する
  - @voluntas
- [ADD] `Error` に `code()` と `function()` のアクセサを追加する
  - @voluntas
- [ADD] Opus ライブラリのバージョン文字列を取得する `version_string()` を追加する
  - @voluntas
- [ADD] デコーダーに FEC デコード機能 `Decoder::decode_fec()` を追加する
  - @voluntas
- [ADD] デコーダーにパケットロス補間機能 `Decoder::decode_plc()` を追加する
  - @voluntas
- [ADD] エンコーダーとデコーダーに状態リセット機能 `reset()` を追加する
  - @voluntas
- [ADD] エンコーダーに `frame_samples()` を追加する
  - @voluntas
- [CHANGE] エンコーダーのコンストラクタを `Encoder::new(sample_rate, channels, bitrate)` から `Encoder::new(EncoderConfig)` に変更する
  - @voluntas
- [CHANGE] デコーダーのコンストラクタを `Decoder::new(sample_rate, channels)` から `Decoder::new(DecoderConfig)` に変更する
  - @voluntas
- [CHANGE] `sample_rate` の型を `u16` から `u32` に変更する
  - @voluntas
- [CHANGE] `Encoder::encode()` の戻り値を `&[u8]` から `Vec<u8>` に変更する
  - @voluntas
- [CHANGE] `Decoder::decode()` の戻り値を `&[i16]` から `Vec<i16>` に変更する
  - @voluntas
- [CHANGE] ビルドシステムを autoreconf/configure/make から shiguredo_cmake に変更する
  - @voluntas
- [FIX] エンコードバッファサイズを RFC 6716 の推奨最大パケットサイズ (4000 バイト) に合わせる
  - @voluntas
- [FIX] エンコーダー / デコーダー生成時の CTL エラーで確実にリソースを解放するように修正する
  - @voluntas
- [FIX] デコーダーの `decode()` で `opus_decode` の実際の戻り値を使用するように修正する
  - @voluntas

### misc

- `cargo-fuzz` による fuzz テストターゲットを追加する
  - @voluntas
- Hisui 固有の記述を削除して OSS 公開用に変更する
  - @voluntas
- build.rs で利用する toml crate を shiguredo_toml crate に置き換える
  - @sile

## 2025.1.0

**リリース日**: 2025-09-26
