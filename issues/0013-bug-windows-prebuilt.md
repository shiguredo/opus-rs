# Windows の prebuilt で libopus.a が見つからずビルドに失敗する

- Created: 2026-10-08
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-windows-prebuilt
- Polished: {YYYY-MM-DD}

## 目的

Windows (x86_64-pc-windows-msvc) で prebuilt を使ったビルドが失敗せずに通るようにする。

## 現状

- `build.rs` の `download_prebuilt` が、prebuilt アーカイブ内のライブラリ名を `libopus.a` に固定してコピーしている
- Windows 向けの prebuilt アーカイブ `libopus-windows_x86_64.tar.gz` には `lib/opus.lib` しか入っていない (`release.yml` の "Create prebuilt archive" が `libopus.a` が無い場合に `opus.lib` を収録している)
- このため Windows で source-build feature なし (prebuilt 経路) のビルドが `failed to copy libopus.a` で panic する
- 再現: momo-rs の Windows CI が `shiguredo_opus v2026.2.0` と `v2026.3.0` のビルドで失敗した (`failed to copy libopus.a: Os { code: 2, kind: NotFound }`)
- 同じ `build.rs` の source-build 経路は `find_static_library` で `libopus.a` と `opus.lib` の両方を扱えている
- CI は全 OS で `--features source-build` を指定しているため、prebuilt 経路が検証されていない

## 設計方針

- `download_prebuilt` で `find_static_library` を使い、見つかったライブラリをファイル名を変えずに `OUT_DIR/lib` へコピーする
  - Windows では `rustc-link-lib=static=opus` が `opus.lib` を探すため、名前を変えるとリンクできない
- 再発防止として release workflow の `build-prebuilt` で、アップロードした prebuilt を取得して source-build なしの `cargo test` を実行する

## 完了条件

- Windows で prebuilt 経路の `cargo build` / `cargo test` が成功する
- Linux / macOS の prebuilt 経路が従来どおり動作する
- release workflow で prebuilt 経路の検証が実行される
