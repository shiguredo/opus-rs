# iOS / Android 向けの prebuilt を追加する

- Created: 2026-10-01
- Completed: {YYYY-MM-DD}
- Branch: feature/add-mobile-prebuilt
- Polished: 2026-10-01

## 目的

ユーザーからの「prebuilt に iOS / Android 向けを用意したい」という要望に対応する。
モバイル向けの Rust アプリケーションでも、Opus の C ライブラリをソースからビルドせずに利用できるようにする。

## 現状

`develop` ブランチの `build.rs` の `get_target_platform` と `.github/workflows/release.yml` の `build-prebuilt` は、Ubuntu、macOS、Windows 向けのみを扱う。
iOS の実機とシミュレーターの区別、Android の ABI の選択、モバイル向けの SDK 設定が必要になる。
また、`rewrite_symbols` は Mach-O のシンボル先頭の `_` を macOS の場合だけ処理している。

本 issue は、`feature/add-mobile-prebuilt` 上で実装とローカル検証を進めた後に起票する。
現時点の変更は未コミットで、GitHub Actions 上の実行とリリース公開は行っていない。

## 設計方針

### 対象ターゲット

| 対象 | Rust ターゲット | prebuilt のプラットフォーム名 |
| --- | --- | --- |
| iOS 実機 arm64 | `aarch64-apple-ios` | `ios_arm64` |
| iOS シミュレーター arm64 | `aarch64-apple-ios-sim` | `ios-sim_arm64` |
| iOS シミュレーター x86_64 | `x86_64-apple-ios` | `ios-sim_x86_64` |
| Android arm64-v8a | `aarch64-linux-android` | `android_arm64` |
| Android armeabi-v7a | `armv7-linux-androideabi` | `android_armv7` |
| Android x86 | `i686-linux-android` | `android_x86` |
| Android x86_64 | `x86_64-linux-android` | `android_x86_64` |

- iOS の prebuilt は 13.0 以降、Android は API level 21 以降を対象とする
- Android のビルドには NDK `28.2.13676358` を使用する
- DRED を有効にし、シンボル書き換え済みの静的ライブラリと対象ターゲット用の `bindings.rs` を配布する
- アーカイブには Opus の `COPYING` を同梱し、SHA256 チェックサムを添付する
- CI とリリースから同じモバイル向けビルドワークフローを呼び出す
- ソースビルドと bindgen で SDK、ABI、最小 OS バージョンを揃える
- モバイル向けの成果物を初めて含むリリース以降、Cargo のターゲットから自動選択する

### 変更対象

- `build.rs` の `get_target_platform`、`configure_mobile_build`、`rewrite_symbols`、`build_symbol_rename_maps`
- `Cargo.toml` の `build-dependencies` と `Cargo.lock`
- `.github/workflows/mobile.yml` の `build`
- `.github/workflows/ci.yml` の `mobile`
- `.github/workflows/release.yml` の `build-mobile-prebuilt` と `publish` の依存関係
- `README.md` のビルド手順と `CHANGES.md` の追加内容

## 完了条件

- 全 7 ターゲットで DRED を含む静的ライブラリとバインディングを生成できる
- 全定義済み外部シンボルが、Mach-O 固有の先頭 `_` を除いて `shiguredo_opus_` プレフィックスを持つ
- 各アーカイブの SHA256 が一致し、収録したライブラリとバインディングを用いて Rust のリンクが成功する
- GitHub Actions の CI でモバイル向けビルドとリンクの検証が通る
- 次回リリースでアーカイブとチェックサムのアップロード、公開された prebuilt の自動選択とリンクの検証、crates.io への公開の順に進むワークフローを構成し、検証に失敗した場合は `publish` を開始しない
- 既存の単体テスト、PBT、フォーマット、Clippy が通る

リリースについては、上記の順序と依存関係を保証するワークフローの構成を本 issue の完了対象とする。
公開されたリリース成果物を取得する経路と `publish` の依存制御は、次回リリースで確認する。

## 解決方法

未コミットの変更では、Rust のターゲットからモバイル向けの成果物を選択し、iOS の実機とシミュレーターを区別する処理を追加した。
Mach-O のシンボル書き換えは Apple プラットフォームを判定するように変更した。

ソースビルドでは、iOS の Xcode SDK と Android NDK のツールチェーンを明示し、bindgen にも同じ SDK とターゲットを指定する。
NDK の C / C++ コンパイラを CMake に渡すため、既存の推移的依存である `cc` をビルド依存として明示した。

armeabi-v7a では、Opus 1.6.1 の DRED と NEON の実行時選択を組み合わせると `DNN_COMPUTE_LINEAR_IMPL` が未定義になることを確認した。
NDK の既定設定に合わせて `OPUS_PRESUME_NEON=ON` を指定し、NEON 対応 CPU を対象とする。

`.github/workflows/mobile.yml` を CI とリリースで共用し、ソースビルド、Rust のリンク、シンボル検証、アーカイブ生成を行う。
リリースから呼び出した場合はアップロード後の prebuilt も検証し、成功を `publish` の前提とする。

### ローカル検証結果

- iOS の 3 ターゲットと Android の 4 ABI でソースビルドと Rust のテスト実行ファイルのリンクに成功した
- ワークフローのアーカイブ生成処理を使い、全 7 ターゲットのシンボル、SHA256、展開した成果物からの再リンクを検証した
- macOS の単体テスト 50 件と PBT 5 件が通過した
- `cargo fmt --all --check`、Clippy、`git diff --check` が通過した
- actionlint は、既存の Ubuntu 26.04 の runner ラベルを一時設定で補って通過した

モバイルの実機やエミュレーターでのテスト実行、GitHub Actions 上の CI、公開したリリース成果物の取得は未検証である。
