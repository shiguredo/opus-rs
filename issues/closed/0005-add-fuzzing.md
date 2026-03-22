# Fuzzing ターゲットを追加する

Created: 2026-03-22
Completed: 2026-03-22
Model: Opus 4.6

## 概要

`cargo-fuzz` による fuzz テストターゲットを追加する。

## 背景

opus-rs は unsafe コードを含む FFI バインディングである。
不正な入力によるメモリ安全性の問題がないことを保証するために、
Fuzzing は正式リリース前に必須。

opus 本体には `opus_decode_fuzzer.c` が存在するが、
Rust バインディング側のラッパーコード (バッファサイズ計算、スライス変換等) が
不正入力で問題を起こさないことを個別に確認する必要がある。

## 対応内容

### fuzz ターゲット

- `fuzz_decode` - ランダムバイト列を `Decoder::decode()` に渡す
  - サンプルレート・チャンネル数もファジング入力から決定する
- `fuzz_decode_f32` - ランダムバイト列を `Decoder::decode_f32()` に渡す
- `fuzz_decode_i24` - ランダムバイト列を `Decoder::decode_i24()` に渡す
- `fuzz_encode_decode` - ランダム PCM データをエンコード→デコードするラウンドトリップ
  - i16, f32, i24 の全エンコード形式をカバー
- `fuzz_decode_sequence` - 複数パケットの連続デコード (decode, decode_fec, decode_plc, f32, i24, reset の組み合わせ)

### 確認すること

- パニックしないこと
- メモリ安全性 (AddressSanitizer で検出可能な問題がないこと)
- Result::Err が返ることは許容する (クラッシュしないことが重要)

### ディレクトリ構成

```
fuzz/
├── Cargo.toml
└── fuzz_targets/
    ├── fuzz_decode.rs
    ├── fuzz_decode_f32.rs
    ├── fuzz_decode_i24.rs
    ├── fuzz_encode_decode.rs
    └── fuzz_decode_sequence.rs
```

## 解決方法

`cargo-fuzz` + `libfuzzer-sys` + `arbitrary` を使って 4 つの fuzz ターゲットを作成した。

- `fuzz_decode`: ランダムバイト列の i16 デコード。サンプルレート・チャンネル数・フレーム時間もファジング入力から決定。
- `fuzz_decode_f32`: ランダムバイト列の f32 デコード。
- `fuzz_decode_i24`: ランダムバイト列の i24 デコード。
- `fuzz_encode_decode`: ランダム PCM データのエンコード→デコードラウンドトリップ。i16, f32, i24 の全形式をカバー。
- `fuzz_decode_sequence`: decode, decode_fec, decode_plc, f32, i24, reset の操作を最大 32 回のランダムシーケンスで実行。

全ターゲットが 10 秒間のテスト実行でクラッシュなく動作することを確認済み。
