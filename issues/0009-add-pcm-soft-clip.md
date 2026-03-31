# PCM ソフトクリッピングを追加する

Created: 2026-03-22
Model: Opus 4.6

## 概要

`opus_pcm_soft_clip` の Rust API を追加する。

## 背景

`opus_pcm_soft_clip` は f32 PCM データに対してソフトクリッピングを適用する。
Opus デコーダーの f32 出力は +/-1.0 の範囲を超える場合があり、
DAC やオーディオ出力に渡す前にクリッピングが必要になることがある。
ハードクリッピング (単純な clamp) と異なり、歪みを軽減する。

## 対応内容

### モジュール関数

- `pcm_soft_clip(pcm: &mut [f32], frame_size: usize, channels: u8, softclip_mem: &mut [f32])` - f32 PCM にソフトクリッピングを適用する

### テスト

- +/-1.0 を超えるサンプルがクリッピングされることの確認
- クリッピング不要なデータが変更されないことの確認
