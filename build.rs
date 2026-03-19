use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

// 依存ライブラリの名前
const LIB_NAME: &str = "opus";

// シンボル書き換え用のプレフィックス
//
// prebuilt で配布する際、他のライブラリが同じ Opus シンボル (opus_encode, opus_decode 等) を
// 使っていると衝突する。この定数のプレフィックスを付けることで回避する。
//
// 変換例:
//   opus_encode  → shiguredo_opus_encode  (opus_ を shiguredo_opus_ に置換)
//   celt_fir     → shiguredo_opus_celt_fir (内部シンボルは単純にプレフィックス付与)
const SYMBOL_PREFIX: &str = "shiguredo_opus";

fn main() {
    // Cargo.toml か build.rs が更新されたら、依存ライブラリを再ビルドする
    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_SOURCE_BUILD");
    println!("cargo::rerun-if-env-changed=OPUS_TARGET");

    // 各種変数やビルドディレクトリのセットアップ
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("infallible"));
    let output_metadata_path = out_dir.join("metadata.rs");
    let output_bindings_path = out_dir.join("bindings.rs");

    // 各種メタデータを書き込む
    let (git_url, version) = get_git_url_and_version();
    fs::write(
        output_metadata_path,
        format!(
            concat!(
                "pub const BUILD_METADATA_REPOSITORY: &str={:?};\n",
                "pub const BUILD_METADATA_VERSION: &str={:?};\n",
            ),
            git_url, version
        ),
    )
    .expect("failed to write metadata file");

    if env::var("DOCS_RS").is_ok() {
        // Docs.rs 向けのビルドでは git clone ができないので build.rs の処理はスキップして、
        // 代わりに、ドキュメント生成時に最低限必要な構造体だけをダミーで出力している。
        //
        // シンボル書き換えもスキップされる（ビルド自体が行われないため）。
        //
        // See also: https://docs.rs/about/builds
        fs::write(
            output_bindings_path,
            "pub struct OpusEncoder; pub struct OpusDecoder;",
        )
        .expect("write file error");
        return;
    }

    let output_lib_dir = if should_use_prebuilt() {
        download_prebuilt(&out_dir)
    } else {
        build_from_source(&out_dir, &output_bindings_path)
    };

    println!("cargo::rustc-link-search={}", output_lib_dir.display());
    println!("cargo::rustc-link-lib=static={LIB_NAME}");
}

// source-build feature が有効でなければ prebuilt を使う
fn should_use_prebuilt() -> bool {
    if env::var("CARGO_FEATURE_SOURCE_BUILD").is_ok() {
        return false;
    }
    true
}

// prebuilt バイナリをダウンロードして展開する
fn download_prebuilt(out_dir: &Path) -> PathBuf {
    let target = get_target_platform();
    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is not set");
    let base_url = format!(
        "https://github.com/shiguredo/opus-rs/releases/download/{}",
        version
    );
    let archive_name = format!("libopus-{}.tar.gz", target);
    let archive_url = format!("{}/{}", base_url, archive_name);
    let sha256_url = format!("{}/{}.sha256", base_url, archive_name);

    let archive_path = out_dir.join("prebuilt.tar.gz");
    let sha256_path = out_dir.join("prebuilt.sha256");
    let prebuilt_dir = out_dir.join("prebuilt");
    fs::create_dir_all(&prebuilt_dir).expect("failed to create prebuilt directory");

    // curl でアーカイブをダウンロード
    eprintln!("prebuilt ライブラリをダウンロード中: {}", archive_url);
    let status = Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(&archive_path)
        .arg(&archive_url)
        .status()
        .expect("failed to execute curl. Ensure curl is installed");
    if !status.success() {
        panic!("failed to download prebuilt library: {}", archive_url);
    }

    // curl で SHA256 チェックサムをダウンロード
    let status = Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(&sha256_path)
        .arg(&sha256_url)
        .status()
        .expect("failed to execute curl");
    if !status.success() {
        panic!("failed to download SHA256 checksum: {}", sha256_url);
    }

    // SHA256 を検証
    verify_sha256(&archive_path, &sha256_path);

    // tar で展開
    let status = Command::new("tar")
        .args(["xzf"])
        .arg(&archive_path)
        .arg("-C")
        .arg(&prebuilt_dir)
        .status()
        .expect("failed to execute tar. Ensure tar is installed");
    if !status.success() {
        panic!("failed to extract prebuilt archive");
    }

    // ライブラリファイルを OUT_DIR/lib/ にコピー
    let lib_dir = out_dir.join("lib");
    fs::create_dir_all(&lib_dir).expect("failed to create lib directory");
    fs::copy(
        prebuilt_dir.join("lib").join("libopus.a"),
        lib_dir.join("libopus.a"),
    )
    .expect("failed to copy libopus.a");

    // 静的ライブラリのシンボルを書き換える
    //
    // prebuilt バイナリも他のライブラリとシンボルが衝突しないよう、
    // source-build と同じプレフィックスを付与する。
    let callbacks = rewrite_symbols(&lib_dir, out_dir);

    // bindings.rs を OUT_DIR/ にコピーし、#[link_name] 属性を挿入する
    //
    // prebuilt では bindgen を実行しないため、配布済みの bindings.rs を後処理して
    // シンボル書き換え後の名前を #[link_name] 属性として挿入する。
    let bindings_src = prebuilt_dir.join("bindings.rs");
    let bindings_dst = out_dir.join("bindings.rs");
    fs::copy(&bindings_src, &bindings_dst).expect("failed to copy bindings.rs");
    inject_link_name_into_bindings(&bindings_dst, &callbacks.rename_map);

    lib_dir
}

// SHA256 チェックサムを検証する
fn verify_sha256(file_path: &Path, sha256_path: &Path) {
    let expected = fs::read_to_string(sha256_path)
        .expect("failed to read SHA256 checksum file")
        .split_whitespace()
        .next()
        .expect("SHA256 checksum file is empty")
        .to_lowercase();

    let actual = compute_sha256(file_path);
    if actual != expected {
        panic!(
            "SHA256 checksum mismatch:\n  expected: {}\n  actual:   {}",
            expected, actual
        );
    }
    eprintln!("SHA256 checksum verified: {}", actual);
}

// ファイルの SHA256 ハッシュを計算する
fn compute_sha256(path: &Path) -> String {
    let output = if cfg!(target_os = "macos") {
        // macOS: shasum を使用
        Command::new("shasum")
            .args(["-a", "256"])
            .arg(path)
            .output()
            .expect("failed to execute shasum. Ensure shasum is installed")
    } else if cfg!(target_os = "windows") {
        // Windows: certutil を使用
        Command::new("certutil")
            .args(["-hashfile"])
            .arg(path)
            .arg("SHA256")
            .output()
            .expect("failed to execute certutil")
    } else {
        // Linux: sha256sum を使用
        Command::new("sha256sum")
            .arg(path)
            .output()
            .expect("failed to execute sha256sum. Ensure coreutils is installed")
    };

    if !output.status.success() {
        panic!("failed to compute SHA256 checksum");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if cfg!(target_os = "windows") {
        // certutil 出力形式:
        // SHA256 hash of <file>:
        // <hash>
        // CertUtil: -hashfile command completed successfully.
        stdout
            .lines()
            .nth(1)
            .expect("unexpected certutil output format")
            .trim()
            .to_lowercase()
    } else {
        // shasum / sha256sum 出力形式: <hash>  <filename>
        stdout
            .split_whitespace()
            .next()
            .expect("unexpected shasum/sha256sum output format")
            .to_lowercase()
    }
}

// ソースからビルドする
fn build_from_source(out_dir: &Path, output_bindings_path: &Path) -> PathBuf {
    let out_build_dir = out_dir.join("build/");
    let src_dir = out_build_dir.join(LIB_NAME);
    let input_header_path = src_dir.join("include/opus.h");
    let _ = fs::remove_dir_all(&out_build_dir);
    fs::create_dir(&out_build_dir).expect("failed to create build directory");

    // 依存ライブラリのリポジトリを取得する
    git_clone_external_lib(&out_build_dir);

    // 依存ライブラリを shiguredo_cmake でビルドする
    shiguredo_cmake::set_cmake_env();
    // profile("Release") を使用: Windows の Visual Studio ジェネレーター (マルチ構成) では
    // CMAKE_BUILD_TYPE が無視されるため、cmake crate の profile() で統一的に指定する
    let dst = shiguredo_cmake::Config::new(&src_dir)
        .profile("Release")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("OPUS_BUILD_TESTING", "OFF")
        .define("OPUS_BUILD_PROGRAMS", "OFF")
        .build();

    let output_lib_dir = dst.join("lib/");

    // 静的ライブラリのシンボルを書き換える
    //
    // ビルドフロー:
    //   1. llvm-nm で静的ライブラリ内の定義済み外部シンボルを収集する
    //   2. 収集したシンボルに対して SYMBOL_PREFIX を付与したリネームマップを生成する
    //   3. llvm-objcopy --redefine-syms でライブラリ内のシンボルを書き換える
    //   4. bindgen に渡す ParseCallbacks を返す（#[link_name] で書き換え後の名前にリンクする）
    //
    // lib.rs 側の変更は不要。bindgen が生成する #[link_name] 属性で透過的に動作する。
    let callbacks = rewrite_symbols(&output_lib_dir, out_dir);

    // バインディングを生成する
    //
    // parse_callbacks にシンボル書き換え用の ParseCallbacks を渡すことで、
    // 生成されるバインディングに #[link_name = "書き換え後のシンボル名"] が自動付与される。
    bindgen::Builder::default()
        .header(input_header_path.to_str().expect("invalid header path"))
        .parse_callbacks(Box::new(callbacks))
        .generate()
        .expect("failed to generate bindings")
        .write_to_file(output_bindings_path)
        .expect("failed to write bindings");

    output_lib_dir
}

// --- シンボル書き換え ---
//
// 他のライブラリとのシンボル衝突を回避するため、静的ライブラリ内の全シンボルに
// プレフィックスを付与する仕組み。
//
// llvm-nm / llvm-objcopy は rustup の llvm-tools コンポーネントに含まれるものを使用する。
// rust-toolchain.toml に components = ["llvm-tools"] の記載が必要。
//
// プラットフォームごとのシンボル形式の違い:
//   - macOS (Mach-O): シンボル先頭に `_` が付く (例: _opus_encode)
//   - Linux (ELF): 先頭 `_` なし (例: opus_encode)
//   - Windows x64 (COFF): 先頭 `_` なし (例: opus_encode)
//
// bindgen の generated_link_name_override は返した文字列に \u{1} プレフィックスを
// 自動付加する。\u{1} はコンパイラに「この名前をそのまま使え（マングリングするな）」と
// 指示するため、プラットフォーム固有のシンボル名（macOS なら _shiguredo_opus_encode）を
// そのまま返す必要がある。

/// llvm-nm / llvm-objcopy のパスを保持する
struct LlvmTools {
    nm: PathBuf,
    objcopy: PathBuf,
}

/// objcopy 用と bindgen 用の 2 つのリネームマップを保持する
///
/// 2 つのマップが必要な理由:
///   - objcopy_map: ライブラリ内の実シンボル名を書き換えるため、プラットフォーム依存の名前を使う
///   - bindgen_map: Rust コードからリンクする際の名前を指定するため、C シンボル名をキーにする
struct SymbolRenameMaps {
    /// llvm-objcopy の --redefine-syms 用マップ
    ///
    /// キー: 元のシンボル名 (例: macOS なら _opus_encode、Linux なら opus_encode)
    /// 値: 書き換え後のシンボル名 (例: macOS なら _shiguredo_opus_encode)
    objcopy_map: HashMap<String, String>,

    /// bindgen の #[link_name] 用マップ
    ///
    /// キー: C シンボル名 (プラットフォーム非依存、例: opus_encode)
    /// 値: 書き換え後のシンボル名 (プラットフォーム依存、例: macOS なら _shiguredo_opus_encode)
    ///
    /// bindgen は \u{1} プレフィックスを付加してマングリングを抑制するため、
    /// 値にはプラットフォーム固有のシンボル名を格納する必要がある。
    bindgen_map: HashMap<String, String>,
}

/// bindgen の ParseCallbacks 実装
///
/// バインディング生成時に、書き換え後のシンボル名を `#[link_name = "..."]` として付与する。
/// これにより lib.rs 側のコード変更なしでシンボル書き換えが透過的に動作する。
#[derive(Debug)]
struct SymbolLinkNameCallbacks {
    /// C シンボル名 → 書き換え後シンボル名のマップ
    rename_map: HashMap<String, String>,
}

impl bindgen::callbacks::ParseCallbacks for SymbolLinkNameCallbacks {
    /// bindgen がバインディングを生成する際に呼ばれるコールバック
    ///
    /// 戻り値が Some の場合、bindgen は #[link_name = "\u{1}<戻り値>"] を生成する。
    /// \u{1} プレフィックスによりコンパイラのシンボルマングリングが抑制されるため、
    /// 戻り値にはプラットフォーム固有のシンボル名を返す必要がある。
    fn generated_link_name_override(
        &self,
        item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        self.rename_map.get(item_info.name).cloned()
    }
}

/// 静的ライブラリのシンボルを書き換え、bindgen 用の ParseCallbacks を返す
///
/// 処理の流れ:
///   1. rustup の sysroot から llvm-nm / llvm-objcopy を探す
///   2. llvm-nm で静的ライブラリの定義済み外部シンボルを収集する
///   3. 収集したシンボルに対してリネームマップを生成する
///   4. マップファイルを書き出し、llvm-objcopy でライブラリ内のシンボルを書き換える
///   5. bindgen 用の ParseCallbacks を返す
fn rewrite_symbols(lib_dir: &Path, out_dir: &Path) -> SymbolLinkNameCallbacks {
    let tools = discover_llvm_tools();
    let lib_path = find_static_library(lib_dir);

    // macOS の Mach-O ではシンボル先頭に `_` が付くため、
    // プラットフォーム判定してリネームマップの生成時に考慮する
    let is_macos = env::var("CARGO_CFG_TARGET_OS")
        .map(|v| v == "macos")
        .unwrap_or(false);

    // シンボル名の変換ルール
    //
    // opus_ プレフィックスを持つシンボル (公開 API) は opus_ を SYMBOL_PREFIX_ に置換する。
    //   例: opus_encode → shiguredo_opus_encode
    //
    // それ以外のシンボル (celt_*, silk_*, ec_* 等の内部シンボル) は先頭に SYMBOL_PREFIX_ を付与する。
    //   例: celt_fir → shiguredo_opus_celt_fir
    let rename_symbol = |name: &str| -> Option<String> {
        if let Some(rest) = name.strip_prefix("opus_") {
            Some(format!("{SYMBOL_PREFIX}_{rest}"))
        } else {
            Some(format!("{SYMBOL_PREFIX}_{name}"))
        }
    };

    // 全定義済み外部シンボルを収集してリネームマップを生成する
    let symbols = collect_defined_external_symbols(&tools.nm, &lib_path);
    let maps = build_symbol_rename_maps(&symbols, is_macos, &rename_symbol);

    // マップファイルを書き出してシンボルを書き換える
    let map_file = out_dir.join("symbol_rename_map.txt");
    write_objcopy_rename_map(&maps.objcopy_map, &map_file);
    rewrite_archive_symbols(&tools.objcopy, &lib_path, &map_file);

    SymbolLinkNameCallbacks {
        rename_map: maps.bindgen_map,
    }
}

/// 静的ライブラリのパスを探す
///
/// cmake のビルド結果は Unix 系では libopus.a、Windows では opus.lib として出力される。
fn find_static_library(lib_dir: &Path) -> PathBuf {
    let unix_path = lib_dir.join("libopus.a");
    if unix_path.exists() {
        return unix_path;
    }
    let win_path = lib_dir.join("opus.lib");
    if win_path.exists() {
        return win_path;
    }
    panic!("static library not found in {}", lib_dir.display());
}

/// rustc --print sysroot の結果を取得する
///
/// llvm-tools は rustup が管理する sysroot 配下にインストールされるため、
/// sysroot のパスを取得して llvm-nm / llvm-objcopy の探索に使用する。
fn get_rustc_sysroot() -> PathBuf {
    let output = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .expect("failed to run rustc --print sysroot");
    if !output.status.success() {
        panic!("rustc --print sysroot failed");
    }
    PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("invalid UTF-8")
            .trim(),
    )
}

/// Windows 対応の実行ファイル名を生成する
///
/// Windows では実行ファイルに .exe 拡張子が必要。
fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// rustup の sysroot から llvm-nm / llvm-objcopy を探す
///
/// llvm-tools コンポーネントのバイナリは以下のパスに配置される:
///   <sysroot>/lib/rustlib/<host>/bin/llvm-nm
///   <sysroot>/lib/rustlib/<host>/bin/llvm-objcopy
///
/// rust-toolchain.toml に llvm-tools コンポーネントの記載が必要。
///
/// llvm-nm / llvm-objcopy はホスト上で実行するツールなので、クロスコンパイル時は
/// TARGET ではなく HOST のパスから探す必要がある。
/// 例: Windows CI でホストが x86_64-pc-windows-msvc、ターゲットが x86_64-pc-windows-gnu の場合、
/// llvm-tools は msvc 側にのみインストールされている。
fn discover_llvm_tools() -> LlvmTools {
    let sysroot = get_rustc_sysroot();
    // llvm-tools はホスト上で動作するため HOST を使う。
    // クロスコンパイル時に TARGET を使うと、ホスト側にインストールされた
    // llvm-tools が見つからない。
    let host = env::var("HOST").expect("HOST environment variable not set");
    let tools_dir = sysroot.join("lib/rustlib").join(host).join("bin");

    let nm = tools_dir.join(exe_name("llvm-nm"));
    let objcopy = tools_dir.join(exe_name("llvm-objcopy"));

    if !nm.exists() {
        panic!(
            "llvm-nm not found at {}. Run: rustup component add llvm-tools",
            nm.display()
        );
    }
    if !objcopy.exists() {
        panic!(
            "llvm-objcopy not found at {}. Run: rustup component add llvm-tools",
            objcopy.display()
        );
    }

    LlvmTools { nm, objcopy }
}

/// llvm-nm で静的ライブラリから定義済み外部シンボルを収集する
///
/// llvm-nm のオプション:
///   --defined-only: 定義済みシンボルのみ (未定義シンボルを除外)
///   --extern-only: 外部シンボルのみ (ローカルシンボルを除外)
///   --format=just-symbols: シンボル名のみ出力 (アドレスやタイプを省略)
///
/// 出力にはオブジェクトファイル名 (例: opus.c.o:) も含まれるため、
/// is_c_identifier() でフィルタリングして純粋なシンボル名のみを抽出する。
fn collect_defined_external_symbols(nm_path: &Path, lib_path: &Path) -> Vec<String> {
    let output = Command::new(nm_path)
        .arg("--defined-only")
        .arg("--extern-only")
        .arg("--format=just-symbols")
        .arg(lib_path)
        .output()
        .expect("failed to run llvm-nm");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("llvm-nm failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).expect("llvm-nm output is not valid UTF-8");
    let mut symbols: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| !s.is_empty() && is_c_identifier(s))
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols
}

/// C 識別子として有効かどうかを判定する
///
/// llvm-nm の --format=just-symbols 出力にはオブジェクトファイル名 (opus.c.o: 等) も
/// 含まれるため、この関数で C 識別子のみをフィルタリングする。
///
/// macOS の Mach-O ではシンボル先頭に `_` が付くため、`_` で始まる文字列も受け入れる。
fn is_c_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// objcopy 用と bindgen 用のリネームマップを生成する
///
/// 2 つのマップを生成する理由:
///
/// objcopy_map: ライブラリバイナリ内の実シンボル名を書き換えるためのマップ。
///   macOS では _opus_encode → _shiguredo_opus_encode のようにプラットフォーム固有の
///   `_` プレフィックスを含む形で管理する。
///
/// bindgen_map: Rust バインディングの #[link_name] に使うマップ。
///   キーは C シンボル名 (opus_encode)、値はプラットフォーム固有のシンボル名
///   (_shiguredo_opus_encode) を格納する。
///   bindgen は generated_link_name_override の戻り値に \u{1} を付加してマングリングを
///   抑制するため、プラットフォーム固有の名前を直接返す必要がある。
fn build_symbol_rename_maps(
    symbols: &[String],
    is_macos: bool,
    rename_symbol: &dyn Fn(&str) -> Option<String>,
) -> SymbolRenameMaps {
    let mut objcopy_map = HashMap::new();
    let mut bindgen_map = HashMap::new();

    for sym in symbols {
        // プラットフォーム固有のプレフィックスを除去して C シンボル名を取得する
        //   macOS: _opus_encode → opus_encode
        //   Linux/Windows: opus_encode → opus_encode (変化なし)
        let c_name = if is_macos {
            sym.strip_prefix('_').unwrap_or(sym)
        } else {
            sym.as_str()
        };

        if let Some(new_c_name) = rename_symbol(c_name) {
            // objcopy 用: プラットフォーム固有のプレフィックスを再付与する
            //   macOS: shiguredo_opus_encode → _shiguredo_opus_encode
            //   Linux/Windows: shiguredo_opus_encode → shiguredo_opus_encode (変化なし)
            let new_sym = if is_macos {
                format!("_{new_c_name}")
            } else {
                new_c_name.clone()
            };
            objcopy_map.insert(sym.clone(), new_sym.clone());

            // bindgen 用: generated_link_name_override は \u{1} プレフィックスを付加して
            // シンボル名をそのまま使うため、プラットフォーム固有のシンボル名で管理する
            bindgen_map.insert(c_name.to_string(), new_sym);
        }
    }

    SymbolRenameMaps {
        objcopy_map,
        bindgen_map,
    }
}

/// --redefine-syms 用のマップファイルを書き出す
///
/// ファイル形式は 1 行に "旧シンボル名 新シンボル名" を空白区切りで記述する。
/// llvm-objcopy の --redefine-syms オプションで使用される。
fn write_objcopy_rename_map(map: &HashMap<String, String>, path: &Path) {
    let mut lines: Vec<String> = map
        .iter()
        .map(|(old, new)| format!("{old} {new}"))
        .collect();
    // 出力を決定的にするためソートする
    lines.sort();
    fs::write(path, lines.join("\n")).expect("failed to write symbol rename map");
}

/// llvm-objcopy でアーカイブ内のシンボルを書き換える
///
/// --redefine-syms はマップファイルに従ってシンボル名を一括置換する。
/// ライブラリファイルはインプレースで更新される。
fn rewrite_archive_symbols(objcopy_path: &Path, lib_path: &Path, map_file: &Path) {
    let status = Command::new(objcopy_path)
        .arg("--redefine-syms")
        .arg(map_file)
        .arg(lib_path)
        .status()
        .expect("failed to run llvm-objcopy");
    if !status.success() {
        panic!("llvm-objcopy failed");
    }
}

/// prebuilt の bindings.rs に #[link_name] 属性を挿入する
///
/// prebuilt パスでは bindgen を実行しないため、配布済みの bindings.rs を後処理して
/// シンボル書き換え後の名前を `#[link_name = "\u{1}<シンボル名>"]` として挿入する。
///
/// 処理対象は `pub fn <name>(` パターンにマッチする行。
/// bindgen_map にエントリがある関数に対してのみ #[link_name] を挿入する。
///
/// \u{1} プレフィックスはコンパイラにシンボル名をそのまま使うよう指示するもので、
/// bindgen が generated_link_name_override で生成するのと同じ形式。
fn inject_link_name_into_bindings(bindings_path: &Path, bindgen_map: &HashMap<String, String>) {
    let content = fs::read_to_string(bindings_path).expect("failed to read bindings.rs");
    let mut output = Vec::new();

    for line in content.lines() {
        // `    pub fn opus_encode(` のようなパターンを検出する
        if let Some(fn_name) = extract_extern_fn_name(line)
            && let Some(new_symbol) = bindgen_map.get(fn_name)
        {
            // #[link_name] 属性を挿入する
            // インデントは元の行に合わせる
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            output.push(format!("{indent}#[link_name = \"\\u{{1}}{new_symbol}\"]"));
        }
        output.push(line.to_string());
    }

    fs::write(bindings_path, output.join("\n")).expect("failed to write bindings.rs");
}

/// `pub fn <name>(` パターンから関数名を抽出する
///
/// bindgen が生成する extern ブロック内の関数宣言を検出するためのヘルパー。
/// 戻り値は関数名 (例: "opus_encode")。マッチしない場合は None。
fn extract_extern_fn_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("pub fn ")?;
    let end = rest.find('(')?;
    Some(&rest[..end])
}

// --- プラットフォーム判定 ---

// CARGO_CFG_TARGET_OS + CARGO_CFG_TARGET_ARCH からプラットフォーム名を生成する
fn get_target_platform() -> String {
    if let Ok(target) = env::var("OPUS_TARGET") {
        return target;
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    match (target_os.as_str(), target_arch.as_str()) {
        ("linux", "x86_64") => format!("{}_x86_64", detect_linux_distro()),
        ("linux", "aarch64") => format!("{}_armv8", detect_linux_distro()),
        ("macos", "aarch64") => "macos_arm64".to_string(),
        ("windows", "x86_64") => "windows_x86_64".to_string(),
        _ => panic!("unsupported target: os={}, arch={}", target_os, target_arch),
    }
}

// /etc/os-release から Ubuntu バージョンを検出する
fn detect_linux_distro() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(version) = line.strip_prefix("VERSION_ID=") {
                let version = version.trim_matches('"');
                match version {
                    "22.04" | "24.04" => return format!("ubuntu-{}", version),
                    _ => {}
                }
            }
        }
    }
    panic!(
        "unsupported Linux distribution. \
         set OPUS_TARGET environment variable to specify the target explicitly"
    );
}

// --- 既存のヘルパー関数 ---

// 外部ライブラリのリポジトリを git clone する
fn git_clone_external_lib(build_dir: &Path) {
    let (git_url, version) = get_git_url_and_version();
    let success = Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--branch")
        .arg(version)
        .arg(git_url)
        .current_dir(build_dir)
        .status()
        .is_ok_and(|status| status.success());
    if !success {
        panic!("failed to clone {LIB_NAME} repository");
    }
}

// Cargo.toml から依存ライブラリの Git URL とバージョンタグを取得する
fn get_git_url_and_version() -> (String, String) {
    let cargo_toml =
        shiguredo_toml::from_str(include_str!("Cargo.toml")).expect("failed to parse Cargo.toml");
    let deps = cargo_toml
        .get("package")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("external-dependencies"))
        .and_then(|v| v.get(LIB_NAME));
    if let Some(dep) = deps {
        let git_url = dep
            .get("git")
            .and_then(|s| s.as_str())
            .expect("missing 'git' field in external-dependencies");
        let version = dep
            .get("version")
            .and_then(|s| s.as_str())
            .expect("missing 'version' field in external-dependencies");
        (git_url.to_string(), version.to_string())
    } else {
        panic!(
            "Cargo.toml does not contain a valid [package.metadata.external-dependencies.{LIB_NAME}] table"
        );
    }
}
