# Fuzzing ターゲットを追加する

Created: 2026-03-22
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
- `fuzz_decode_float` - ランダムバイト列を `Decoder::decode_float()` に渡す (0001 完了後)
- `fuzz_encode_decode` - ランダム i16 PCM データをエンコード→デコードするラウンドトリップ
- `fuzz_decode_sequence` - 複数パケットの連続デコード (decode, decode_fec, decode_plc の組み合わせ)

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
    ├── fuzz_decode_float.rs
    ├── fuzz_encode_decode.rs
    └── fuzz_decode_sequence.rs
```
