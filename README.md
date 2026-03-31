# opus-rs

[![crates.io](https://img.shields.io/crates/v/shiguredo_opus.svg)](https://crates.io/crates/shiguredo_opus)
[![docs.rs](https://docs.rs/shiguredo_opus/badge.svg)](https://docs.rs/shiguredo_opus)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?logo=discord&logoColor=white)](https://discord.gg/shiguredo)

## About Shiguredo's open source software

We will not respond to PRs or issues that have not been discussed on Discord. Also, Discord is only available in Japanese.

Please read <https://github.com/shiguredo/oss> before use.

## 時雨堂のオープンソースソフトウェアについて

利用前に <https://github.com/shiguredo/oss> をお読みください。

## 概要

[xiph/opus](https://github.com/xiph/opus) を利用した Opus エンコーダーおよびデコーダーの Rust バインディングです。

ビルド時に Opus ライブラリのソースコードを取得し、CMake でスタティックライブラリとしてビルドします。

## 特徴

- Opus エンコード / デコード
- エンコーダーはサンプルレート / チャンネル数 / ビットレート / アプリケーションモード / フレーム時間を指定可能
- VBR / CBR / 帯域幅 / FEC / DTX 等の詳細設定に対応
- デコーダーはサンプルレート / チャンネル数 / フレーム時間 / ゲイン調整を指定可能
- FEC デコード / PLC (パケットロス補間) に対応
- エンコーダー / デコーダーの状態リセットに対応
- MP4 preSkip 値 (lookahead) の取得に対応
- Opus ライブラリのバージョン文字列取得に対応
- 1 フレーム入力 / 1 フレーム出力のシンプルな API
- 静的ライブラリの全シンボルを `shiguredo_opus_` プレフィックス付きに書き換えて他ライブラリとの衝突を回避

## ビルド

```bash
cargo build
```

## 使い方

### エンコード

`EncoderConfig::new()` で必須パラメーターのみ指定し、オプションは struct update syntax で追加できます。

```rust
use shiguredo_opus::{Encoder, EncoderConfig, Application, InbandFec};

// 最小限の設定
let mut encoder = Encoder::new(EncoderConfig {
    bitrate: Some(128_000),
    ..EncoderConfig::new(48000, 2)
})?;

// VoIP 向けの詳細設定
let mut encoder = Encoder::new(EncoderConfig {
    bitrate: Some(64_000),
    application: Some(Application::Voip),
    inband_fec: Some(InbandFec::Enabled),
    packet_loss_perc: Some(10),
    ..EncoderConfig::new(48000, 1)
})?;

// PCM データ (i16, インターリーブ) をエンコード
// 1 フレーム分 (48kHz / 20ms = 960 サンプル × 2ch) を渡す
let pcm: &[i16] = &[0; 960 * 2];
let encoded: Vec<u8> = encoder.encode(pcm)?;

// 1 フレームあたりのサンプル数を取得する
let frame_samples: usize = encoder.frame_samples();

// MP4 の preSkip 値を取得する
let lookahead: u16 = encoder.get_lookahead()?;
```

### デコード

```rust
use shiguredo_opus::{Decoder, DecoderConfig};

let mut decoder = Decoder::new(DecoderConfig::new(48000, 2))?;

// 1 パケット分の圧縮データをデコード (i16, インターリーブ)
let pcm: Vec<i16> = decoder.decode(&encoded_data)?;

// 1 フレームあたりのサンプル数を取得する
let frame_samples: usize = decoder.frame_samples();
```

### FEC デコード

パケットロス発生時に、次のパケットの FEC データから失われたフレームを復元する。
エンコーダー側で `inband_fec` を有効にしておく必要がある。

```rust
// パケット N のロスを検知した場合:
// 1. 次のパケット (N+1) の FEC データから失われたフレームを復元する
let recovered: Vec<i16> = decoder.decode_fec(&packet_n_plus_1)?;

// 2. パケット N+1 を通常デコードする
let decoded: Vec<i16> = decoder.decode(&packet_n_plus_1)?;
```

### PLC (パケットロス補間)

FEC データが利用できない場合のフォールバック。デコーダーの内部状態に基づいて補間フレームを生成する。

```rust
// パケットロス時に補間フレームを生成する
// フレームサイズは DecoderConfig の frame_duration で決定される
let concealed: Vec<i16> = decoder.decode_plc()?;
```

### 状態リセット

エンコーダー / デコーダーの内部状態をリセットする。インスタンスを再生成せずに初期状態に戻せる。

```rust
encoder.reset()?;
decoder.reset()?;
```

### バージョン情報

```rust
use shiguredo_opus;

// リンクされた Opus ライブラリのバージョン文字列を取得する
let version: String = shiguredo_opus::version_string();
// 例: "libopus 1.6.1"

// ビルド時に参照したリポジトリ情報
let repo: &str = shiguredo_opus::BUILD_REPOSITORY;
let tag: &str = shiguredo_opus::BUILD_VERSION;
```

### エラーハンドリング

```rust
use shiguredo_opus::{Encoder, EncoderConfig};

match Encoder::new(EncoderConfig::new(0, 2)) {
    Ok(_) => {}
    Err(e) => {
        // エラーコードと関数名にアクセスできる
        let code: std::ffi::c_int = e.code();
        let function: &str = e.function();
        eprintln!("{e}"); // Display トレイトでフォーマットされたメッセージ
    }
}
```

## 設定

### `EncoderConfig`

#### 必須フィールド

| フィールド | 型 | 説明 |
| --- | --- | --- |
| `sample_rate` | `u32` | サンプルレート (Hz)。8000 / 12000 / 16000 / 24000 / 48000 のいずれか |
| `channels` | `u8` | チャンネル数。1 (モノラル) または 2 (ステレオ) |

#### オプションフィールド

| フィールド | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `bitrate` | `Option<u32>` | Opus 依存 | ビットレート (bps)。500 〜 512000 |
| `application` | `Option<Application>` | `Audio` | アプリケーションモード |
| `frame_duration` | `Option<FrameDuration>` | `Ms20` | フレーム時間 |
| `complexity` | `Option<u8>` | 10 | 計算量 (0-10) |
| `vbr` | `Option<bool>` | `true` | VBR の有効化 |
| `vbr_constraint` | `Option<bool>` | `true` | 制約付き VBR の有効化 |
| `max_bandwidth` | `Option<Bandwidth>` | `Fullband` | 帯域幅の上限 |
| `bandwidth` | `Option<Bandwidth>` | 自動 | 帯域幅の強制指定 |
| `signal` | `Option<Signal>` | 自動 | シグナルタイプのヒント |
| `force_channels` | `Option<ForceChannels>` | 自動 | チャンネル強制設定 |
| `inband_fec` | `Option<InbandFec>` | `Disabled` | 帯域内 FEC モード |
| `packet_loss_perc` | `Option<u8>` | 0 | 想定パケットロス率 (0-100) |
| `dtx` | `Option<bool>` | `false` | DTX (不連続送信) の有効化 |
| `lsb_depth` | `Option<u8>` | 24 | 入力信号の有効ビット深度 (8-24) |
| `prediction_disabled` | `Option<bool>` | `false` | 予測の無効化 |
| `phase_inversion_disabled` | `Option<bool>` | `false` | 位相反転の無効化 |

### `DecoderConfig`

#### 必須フィールド

| フィールド | 型 | 説明 |
| --- | --- | --- |
| `sample_rate` | `u32` | サンプルレート (Hz)。8000 / 12000 / 16000 / 24000 / 48000 のいずれか |
| `channels` | `u8` | チャンネル数。1 (モノラル) または 2 (ステレオ) |

#### オプションフィールド

| フィールド | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `frame_duration` | `Option<FrameDuration>` | `Ms20` | フレーム時間 (PLC で使用) |
| `gain` | `Option<i32>` | 0 | ゲイン調整 (Q8 dB 単位、-32768 〜 32767) |

### enum 型

#### `Application`

| バリアント | 説明 |
| --- | --- |
| `Voip` | 音声通話向け |
| `Audio` | 音楽 / 汎用オーディオ向け |
| `LowDelay` | 低遅延向け |

#### `FrameDuration`

| バリアント | 48kHz でのサンプル数 |
| --- | --- |
| `Ms2_5` | 120 |
| `Ms5` | 240 |
| `Ms10` | 480 |
| `Ms20` | 960 |
| `Ms40` | 1920 |
| `Ms60` | 2880 |

#### `Bandwidth`

| バリアント | 帯域幅 |
| --- | --- |
| `Narrowband` | 4 kHz |
| `Mediumband` | 6 kHz |
| `Wideband` | 8 kHz |
| `Superwideband` | 12 kHz |
| `Fullband` | 20 kHz |

#### `Signal`

| バリアント | 説明 |
| --- | --- |
| `Voice` | 音声信号 |
| `Music` | 音楽信号 |

#### `ForceChannels`

| バリアント | 説明 |
| --- | --- |
| `Mono` | モノラル強制 |
| `Stereo` | ステレオ強制 |

#### `InbandFec`

| バリアント | 説明 |
| --- | --- |
| `Disabled` | FEC 無効 |
| `Enabled` | FEC 有効 |
| `EnabledKeepMusic` | FEC 有効 (音楽時は SILK に切り替えない) |

## Opus ライセンス

<https://github.com/xiph/opus/blob/main/COPYING>

```text
Copyright 2001-2023 Xiph.Org, Skype Limited, Octasic,
                    Jean-Marc Valin, Timothy B. Terriberry,
                    CSIRO, Gregory Maxwell, Mark Borgerding,
                    Erik de Castro Lopo, Mozilla, Amazon

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

- Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.

- Redistributions in binary form must reproduce the above copyright
notice, this list of conditions and the following disclaimer in the
documentation and/or other materials provided with the distribution.

- Neither the name of Internet Society, IETF or IETF Trust, nor the
names of specific contributors, may be used to endorse or promote
products derived from this software without specific prior written
permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER
OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

Opus is subject to the royalty-free patent licenses which are
specified at:

Xiph.Org Foundation:
https://datatracker.ietf.org/ipr/1524/

Microsoft Corporation:
https://datatracker.ietf.org/ipr/1914/

Broadcom Corporation:
https://datatracker.ietf.org/ipr/1526/
```

## ライセンス

Apache License 2.0

```text
Copyright 2026-2026, Shiguredo Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
