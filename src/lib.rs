//! [Opus] エンコーダーとデコーダーの Rust バインディング
//!
//! [xiph/opus](https://github.com/xiph/opus) の Rust バインディング。
//! PCM 音声データのエンコード / デコードを行う。
//!
//! [Opus]: https://github.com/xiph/opus
#![warn(missing_docs)]

use std::ffi::{CStr, c_int};

mod sys;

/// ビルド時に参照したリポジトリ URL
pub const BUILD_REPOSITORY: &str = sys::BUILD_METADATA_REPOSITORY;

/// ビルド時に参照したリポジトリのバージョン（タグ）
pub const BUILD_VERSION: &str = sys::BUILD_METADATA_VERSION;

/// リンクされた Opus ライブラリのバージョン文字列を取得する
///
/// `opus_get_version_string()` の戻り値をそのまま返す。
/// 例: `"libopus 1.6.1"`
pub fn version_string() -> &'static str {
    unsafe {
        let ptr = sys::opus_get_version_string();
        CStr::from_ptr(ptr)
            .to_str()
            .expect("opus_get_version_string returned invalid UTF-8")
    }
}

/// Opus API のエラー
///
/// Opus ライブラリが返すエラーコードをラップする。
/// コード 0 以上は成功を意味し、負の値はエラーとなる。
#[derive(Debug)]
pub struct Error {
    /// Opus API が返したエラーコード
    code: c_int,
    /// エラーが発生した API 関数名
    function: &'static str,
}

impl Error {
    /// エラーコードを検査し、負の値ならエラーを返す
    fn check(code: c_int, function: &'static str) -> Result<(), Self> {
        if code >= 0 {
            Ok(())
        } else {
            Err(Self { code, function })
        }
    }

    /// エラーコードを返す
    ///
    /// Opus API が返した負のエラーコード。
    pub fn code(&self) -> c_int {
        self.code
    }

    /// エラーが発生した API 関数名を返す
    pub fn function(&self) -> &'static str {
        self.function
    }

    /// Opus ライブラリからエラーの理由を取得する
    fn reason(&self) -> Option<&CStr> {
        let reason = unsafe { sys::opus_strerror(self.code) };
        if reason.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(reason) })
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(reason) = self.reason() {
            write!(
                f,
                "[{}] {}() failed: code={}, reason={}",
                env!("CARGO_PKG_NAME"),
                self.function,
                self.code,
                reason.to_string_lossy()
            )
        } else {
            write!(
                f,
                "[{}] {}() failed: code={}",
                env!("CARGO_PKG_NAME"),
                self.function,
                self.code
            )
        }
    }
}

impl std::error::Error for Error {}

// --- 共通の enum 型 ---

/// Opus アプリケーションモード
///
/// エンコーダーの最適化対象を指定する。
/// 用途に応じて適切なモードを選択することで、品質と遅延のバランスを調整できる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Application {
    /// 音声通話向け (OPUS_APPLICATION_VOIP)
    ///
    /// 音声信号に最適化されたモード。
    /// ハイパスフィルタリングやフォルマント強調を行う。
    /// オプションで帯域内前方誤り訂正 (FEC) を含む。
    Voip,
    /// 音楽 / 汎用オーディオ向け (OPUS_APPLICATION_AUDIO)
    ///
    /// 音楽や混合コンテンツに最適化されたモード。
    /// 15ms 未満のコーディング遅延が必要な場合にも適している。
    Audio,
    /// 低遅延向け (OPUS_APPLICATION_RESTRICTED_LOWDELAY)
    ///
    /// 音声最適化モードを無効にして遅延を削減するモード。
    /// 新規またはリセット直後のエンコーダーでのみ設定可能。
    LowDelay,
}

/// フレーム時間
///
/// Opus エンコーダーの 1 フレームあたりの時間を指定する。
/// サンプルレートに応じて 1 フレームあたりのサンプル数が決まる。
///
/// 10ms 未満のフレームサイズでは LPC / ハイブリッドモードが使用できない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDuration {
    /// 2.5 ミリ秒
    Ms2_5,
    /// 5 ミリ秒
    Ms5,
    /// 10 ミリ秒
    Ms10,
    /// 20 ミリ秒
    Ms20,
    /// 40 ミリ秒
    Ms40,
    /// 60 ミリ秒
    Ms60,
}

impl FrameDuration {
    /// 指定サンプルレートでの 1 フレームあたりのサンプル数（チャンネルあたり）を返す
    fn samples_per_frame(self, sample_rate: u32) -> usize {
        match self {
            Self::Ms2_5 => sample_rate as usize / 400,
            Self::Ms5 => sample_rate as usize / 200,
            Self::Ms10 => sample_rate as usize / 100,
            Self::Ms20 => sample_rate as usize / 50,
            Self::Ms40 => sample_rate as usize / 25,
            Self::Ms60 => sample_rate as usize * 3 / 50,
        }
    }
}

/// 帯域幅
///
/// エンコーダーの帯域幅を指定する。
/// [`EncoderConfig::bandwidth`] で強制指定、
/// [`EncoderConfig::max_bandwidth`] で上限として使用できる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bandwidth {
    /// ナローバンド (4 kHz)
    Narrowband,
    /// ミディアムバンド (6 kHz)
    Mediumband,
    /// ワイドバンド (8 kHz)
    Wideband,
    /// スーパーワイドバンド (12 kHz)
    Superwideband,
    /// フルバンド (20 kHz)
    Fullband,
}

impl Bandwidth {
    fn to_opus(self) -> i32 {
        match self {
            Self::Narrowband => sys::OPUS_BANDWIDTH_NARROWBAND as i32,
            Self::Mediumband => sys::OPUS_BANDWIDTH_MEDIUMBAND as i32,
            Self::Wideband => sys::OPUS_BANDWIDTH_WIDEBAND as i32,
            Self::Superwideband => sys::OPUS_BANDWIDTH_SUPERWIDEBAND as i32,
            Self::Fullband => sys::OPUS_BANDWIDTH_FULLBAND as i32,
        }
    }
}

/// シグナルタイプ
///
/// エンコーダーへのヒント。モード選択の閾値に影響する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// 音声信号 (OPUS_SIGNAL_VOICE)
    ///
    /// LPC / ハイブリッドモードを選択しやすくする。
    Voice,
    /// 音楽信号 (OPUS_SIGNAL_MUSIC)
    ///
    /// MDCT モードを選択しやすくする。
    Music,
}

/// チャンネル強制設定
///
/// 入力のチャンネル数にかかわらず、エンコード出力のチャンネル数を強制する。
/// ステレオストリーム内のモノラル音源を扱う場合に有用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceChannels {
    /// モノラル強制
    Mono,
    /// ステレオ強制
    Stereo,
}

/// 帯域内前方誤り訂正 (FEC) モード
///
/// LPC レイヤーにのみ適用される。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InbandFec {
    /// FEC 無効（デフォルト）
    Disabled,
    /// FEC 有効
    ///
    /// パケットロス率が十分に高い場合、高ビットレートでも
    /// 自動的に SILK に切り替えて FEC を使用する。
    Enabled,
    /// FEC 有効（音楽時は SILK に切り替えない）
    EnabledKeepMusic,
}

// --- エンコーダー ---

/// エンコーダーの設定
///
/// Opus エンコーダーの生成に必要なパラメーターを保持する。
/// `Option` のフィールドは未指定時に Opus のデフォルト値が使用される。
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// 入力 PCM のサンプルレート (Hz)
    ///
    /// Opus がサポートするサンプルレートは 8000, 12000, 16000, 24000, 48000 のいずれか。
    /// 0 を指定するとエラーが返る。
    pub sample_rate: u32,

    /// 入力 PCM のチャンネル数
    ///
    /// 1（モノラル）または 2（ステレオ）を指定する。
    /// 0 を指定するとエラーが返る。
    pub channels: u8,

    /// ターゲットビットレート (bps)
    ///
    /// 500 〜 512000 の範囲が有効。
    /// 未指定時はチャンネル数とサンプルレートに基づくデフォルト値が使用される。
    pub bitrate: Option<u32>,

    /// アプリケーションモード
    ///
    /// 未指定時は [`Application::Audio`] が使用される。
    pub application: Option<Application>,

    /// フレーム時間
    ///
    /// 未指定時は [`FrameDuration::Ms20`] が使用される。
    /// 48kHz の場合、Ms20 は 960 サンプル / チャンネルに相当する。
    pub frame_duration: Option<FrameDuration>,

    /// 計算量 (0-10)
    ///
    /// 値が大きいほど高品質だが処理が重い。
    /// 未指定時は Opus のデフォルト値（10）が使用される。
    pub complexity: Option<u8>,

    /// VBR（可変ビットレート）の有効化
    ///
    /// `true` で VBR 有効（デフォルト）、`false` でハード CBR。
    /// CBR は低ビットレートの LPC / ハイブリッドモードで品質劣化を起こすことがある。
    /// 未指定時は Opus のデフォルト値（VBR 有効）が使用される。
    pub vbr: Option<bool>,

    /// 制約付き VBR の有効化
    ///
    /// VBR モード時のみ有効。
    /// `true` で制約付き VBR（デフォルト）。公称ビットレートでのシリアライズ時に
    /// 最大 1 フレーム分のバッファリング遅延を生じさせる。
    /// 未指定時は Opus のデフォルト値（制約付き VBR 有効）が使用される。
    ///
    /// MDCT モードでのみ制約が有効。
    /// SILK モードでは完全に無視され、ハイブリッドモードでは
    /// LPC レイヤーが制約を超えるビットレートを使用する場合がある。
    pub vbr_constraint: Option<bool>,

    /// 最大帯域幅
    ///
    /// エンコーダーが自動選択する帯域幅の上限を設定する。
    /// [`EncoderConfig::bandwidth`] よりもこちらを使用することが推奨される。
    /// ビットレートが低い場合にエンコーダーが自動的に帯域幅を狭める余地を残す。
    /// 未指定時は Opus のデフォルト値（Fullband）が使用される。
    pub max_bandwidth: Option<Bandwidth>,

    /// 帯域幅の強制指定
    ///
    /// エンコーダーの帯域幅を固定する。
    /// 通常は [`EncoderConfig::max_bandwidth`] を使用して上限のみ設定し、
    /// エンコーダーに自動選択させることが推奨される。
    /// 未指定時はエンコーダーがビットレートに基づいて自動選択する。
    pub bandwidth: Option<Bandwidth>,

    /// シグナルタイプのヒント
    ///
    /// エンコーダーのモード選択閾値に影響するヒント。
    /// 未指定時はエンコーダーが自動判定する。
    pub signal: Option<Signal>,

    /// チャンネル強制設定
    ///
    /// 入力のチャンネル数にかかわらず、エンコード出力のチャンネル数を強制する。
    /// 未指定時はエンコーダーが自動選択する。
    pub force_channels: Option<ForceChannels>,

    /// 帯域内前方誤り訂正 (FEC) モード
    ///
    /// LPC レイヤーにのみ適用される。
    /// 未指定時は Opus のデフォルト値（FEC 無効）が使用される。
    pub inband_fec: Option<InbandFec>,

    /// 想定パケットロス率 (0-100)
    ///
    /// 値が大きいほどロス耐性が向上するが、ロスがない場合の品質は低下する。
    /// 未指定時は Opus のデフォルト値（0）が使用される。
    pub packet_loss_perc: Option<u8>,

    /// DTX（不連続送信）の有効化
    ///
    /// LPC レイヤーにのみ適用される。
    /// `true` で DTX 有効。無音区間でコンフォートノイズフレームを送信する。
    /// 未指定時は Opus のデフォルト値（DTX 無効）が使用される。
    pub dtx: Option<bool>,

    /// 入力信号の有効ビット深度 (8-24)
    ///
    /// 無音・微小信号の検出に使用されるヒント。
    /// 例: G.711 u-law 入力なら 14、16bit リニア PCM なら 16。
    /// opus_encode() 使用時は指定値と 16 の小さい方が使用される。
    /// 未指定時は Opus のデフォルト値（24）が使用される。
    pub lsb_depth: Option<u8>,

    /// 予測の無効化
    ///
    /// `true` で予測をほぼ完全に無効化し、フレームをほぼ独立にする。品質は低下する。
    /// 未指定時は Opus のデフォルト値（予測有効）が使用される。
    pub prediction_disabled: Option<bool>,

    /// 位相反転の無効化
    ///
    /// インテンシティステレオの位相反転を無効にする。
    /// `true` でモノラルダウンミックスの品質が向上するが、
    /// 通常のステレオ品質はわずかに低下する。
    /// 未指定時は Opus のデフォルト値（位相反転有効）が使用される。
    pub phase_inversion_disabled: Option<bool>,
}

impl EncoderConfig {
    /// 必須フィールドのみを指定してデフォルト設定を生成する
    ///
    /// オプションフィールドはすべて `None`（Opus デフォルト値）で初期化される。
    pub fn new(sample_rate: u32, channels: u8) -> Self {
        Self {
            sample_rate,
            channels,
            bitrate: None,
            application: None,
            frame_duration: None,
            complexity: None,
            vbr: None,
            vbr_constraint: None,
            max_bandwidth: None,
            bandwidth: None,
            signal: None,
            force_channels: None,
            inband_fec: None,
            packet_loss_perc: None,
            dtx: None,
            lsb_depth: None,
            prediction_disabled: None,
            phase_inversion_disabled: None,
        }
    }
}

/// エンコーダー
///
/// Opus ライブラリを使用して PCM 音声データを Opus フォーマットにエンコードする。
///
/// # 使用フロー
///
/// 1. [`Encoder::new()`] でインスタンスを生成する
/// 2. [`Encoder::encode()`] で 1 フレーム分の PCM データを渡し、エンコード結果を受け取る
#[derive(Debug)]
pub struct Encoder {
    /// Opus エンコーダーのインスタンス
    inner: *mut sys::OpusEncoder,
    /// チャンネル数
    channels: u8,
    /// 1 フレームあたりのサンプル数（チャンネルあたり）
    frame_samples: usize,
    /// エンコード結果を格納するための一時バッファ
    encode_buf: Vec<u8>,
}

/// エンコーダー CTL を設定するヘルパー
unsafe fn set_encoder_ctl(
    inner: *mut sys::OpusEncoder,
    request: u32,
    value: c_int,
    name: &'static str,
) -> Result<(), Error> {
    let code = unsafe { sys::opus_encoder_ctl(inner, request as i32, value) };
    Error::check(code, name)
}

/// エンコーダー生成中の破棄を保証するガード
///
/// `Encoder::new()` 内で CTL 設定中にエラーが発生した場合、
/// このガードが Drop 時にエンコーダーを破棄する。
/// 正常完了時は `defuse()` で破棄を抑制する。
struct EncoderGuard {
    inner: *mut sys::OpusEncoder,
    defused: bool,
}

impl EncoderGuard {
    fn new(inner: *mut sys::OpusEncoder) -> Self {
        Self {
            inner,
            defused: false,
        }
    }

    fn defuse(mut self) -> *mut sys::OpusEncoder {
        self.defused = true;
        self.inner
    }
}

impl Drop for EncoderGuard {
    fn drop(&mut self) {
        if !self.defused {
            unsafe {
                sys::opus_encoder_destroy(self.inner);
            }
        }
    }
}

impl Encoder {
    /// エンコーダーインスタンスを生成する
    ///
    /// Opus エンコーダーを作成し、各種パラメーターを設定する。
    pub fn new(config: EncoderConfig) -> Result<Self, Error> {
        if config.sample_rate == 0 {
            return Err(Error {
                code: sys::OPUS_BAD_ARG,
                function: "Encoder::new(sample_rate)",
            });
        }
        if config.channels == 0 {
            return Err(Error {
                code: sys::OPUS_BAD_ARG,
                function: "Encoder::new(channels)",
            });
        }

        let application = config.application.unwrap_or(Application::Audio);
        let frame_duration = config.frame_duration.unwrap_or(FrameDuration::Ms20);
        let frame_samples = frame_duration.samples_per_frame(config.sample_rate);

        let app_value = match application {
            Application::Voip => sys::OPUS_APPLICATION_VOIP,
            Application::Audio => sys::OPUS_APPLICATION_AUDIO,
            Application::LowDelay => sys::OPUS_APPLICATION_RESTRICTED_LOWDELAY,
        };

        let mut error = 0;
        let inner = unsafe {
            sys::opus_encoder_create(
                config.sample_rate as i32,
                config.channels as c_int,
                app_value as i32,
                &mut error,
            )
        };
        Error::check(error, "opus_encoder_create")?;

        // エラー時にエンコーダーを確実に破棄するためのガード
        let guard = EncoderGuard::new(inner);

        // ビットレート設定
        if let Some(bitrate) = config.bitrate {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_BITRATE_REQUEST,
                    bitrate as c_int,
                    "opus_encoder_ctl(OPUS_SET_BITRATE)",
                )?;
            }
        }

        // 計算量設定
        if let Some(complexity) = config.complexity {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_COMPLEXITY_REQUEST,
                    complexity as c_int,
                    "opus_encoder_ctl(OPUS_SET_COMPLEXITY)",
                )?;
            }
        }

        // VBR 設定
        if let Some(vbr) = config.vbr {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_VBR_REQUEST,
                    vbr as c_int,
                    "opus_encoder_ctl(OPUS_SET_VBR)",
                )?;
            }
        }

        // 制約付き VBR 設定
        if let Some(vbr_constraint) = config.vbr_constraint {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_VBR_CONSTRAINT_REQUEST,
                    vbr_constraint as c_int,
                    "opus_encoder_ctl(OPUS_SET_VBR_CONSTRAINT)",
                )?;
            }
        }

        // 最大帯域幅設定
        if let Some(max_bandwidth) = config.max_bandwidth {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_MAX_BANDWIDTH_REQUEST,
                    max_bandwidth.to_opus() as c_int,
                    "opus_encoder_ctl(OPUS_SET_MAX_BANDWIDTH)",
                )?;
            }
        }

        // 帯域幅強制設定
        if let Some(bandwidth) = config.bandwidth {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_BANDWIDTH_REQUEST,
                    bandwidth.to_opus() as c_int,
                    "opus_encoder_ctl(OPUS_SET_BANDWIDTH)",
                )?;
            }
        }

        // シグナルタイプ設定
        if let Some(signal) = config.signal {
            let value = match signal {
                Signal::Voice => sys::OPUS_SIGNAL_VOICE as c_int,
                Signal::Music => sys::OPUS_SIGNAL_MUSIC as c_int,
            };
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_SIGNAL_REQUEST,
                    value,
                    "opus_encoder_ctl(OPUS_SET_SIGNAL)",
                )?;
            }
        }

        // チャンネル強制設定
        if let Some(force_channels) = config.force_channels {
            let value = match force_channels {
                ForceChannels::Mono => 1,
                ForceChannels::Stereo => 2,
            };
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_FORCE_CHANNELS_REQUEST,
                    value,
                    "opus_encoder_ctl(OPUS_SET_FORCE_CHANNELS)",
                )?;
            }
        }

        // FEC 設定
        if let Some(inband_fec) = config.inband_fec {
            let value = match inband_fec {
                InbandFec::Disabled => 0,
                InbandFec::Enabled => 1,
                InbandFec::EnabledKeepMusic => 2,
            };
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_INBAND_FEC_REQUEST,
                    value,
                    "opus_encoder_ctl(OPUS_SET_INBAND_FEC)",
                )?;
            }
        }

        // パケットロス率設定
        if let Some(packet_loss_perc) = config.packet_loss_perc {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_PACKET_LOSS_PERC_REQUEST,
                    packet_loss_perc as c_int,
                    "opus_encoder_ctl(OPUS_SET_PACKET_LOSS_PERC)",
                )?;
            }
        }

        // DTX 設定
        if let Some(dtx) = config.dtx {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_DTX_REQUEST,
                    dtx as c_int,
                    "opus_encoder_ctl(OPUS_SET_DTX)",
                )?;
            }
        }

        // LSB 深度設定
        if let Some(lsb_depth) = config.lsb_depth {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_LSB_DEPTH_REQUEST,
                    lsb_depth as c_int,
                    "opus_encoder_ctl(OPUS_SET_LSB_DEPTH)",
                )?;
            }
        }

        // 予測無効化設定
        if let Some(prediction_disabled) = config.prediction_disabled {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_PREDICTION_DISABLED_REQUEST,
                    prediction_disabled as c_int,
                    "opus_encoder_ctl(OPUS_SET_PREDICTION_DISABLED)",
                )?;
            }
        }

        // 位相反転無効化設定
        if let Some(phase_inversion_disabled) = config.phase_inversion_disabled {
            unsafe {
                set_encoder_ctl(
                    inner,
                    sys::OPUS_SET_PHASE_INVERSION_DISABLED_REQUEST,
                    phase_inversion_disabled as c_int,
                    "opus_encoder_ctl(OPUS_SET_PHASE_INVERSION_DISABLED)",
                )?;
            }
        }

        // エンコード結果の一時バッファサイズ
        //
        // Opus の推奨最大パケットサイズは 4000 バイト（RFC 6716）。
        // 極端に長いフレーム（60ms ステレオ）の PCM サイズも考慮し、
        // 推奨値と PCM サイズの大きい方を採用する。
        let pcm_size = frame_samples * config.channels as usize * size_of::<i16>();
        let encode_buf_size = 4000.max(pcm_size);

        // すべての CTL 設定が成功したのでガードを解除する
        let inner = guard.defuse();

        Ok(Self {
            inner,
            channels: config.channels,
            frame_samples,
            encode_buf: vec![0u8; encode_buf_size],
        })
    }

    /// MP4 のサンプルエントリーで設定する preSkip の値を取得する
    pub fn get_lookahead(&self) -> Result<u16, Error> {
        let mut value = 0;
        unsafe {
            let code = sys::opus_encoder_ctl(
                self.inner,
                sys::OPUS_GET_LOOKAHEAD_REQUEST as i32,
                &mut value,
            );
            Error::check(code, "opus_encoder_ctl(OPUS_GET_LOOKAHEAD)")?;
        }
        Ok(value as u16)
    }

    /// 1 フレームあたりのサンプル数（チャンネルあたり）を返す
    pub fn frame_samples(&self) -> usize {
        self.frame_samples
    }

    /// エンコーダーの内部状態をリセットする
    ///
    /// インスタンスを再生成せずに初期状態に戻す。
    /// CTL で設定した値はそのまま保持される。
    pub fn reset(&mut self) -> Result<(), Error> {
        unsafe {
            let code = sys::opus_encoder_ctl(self.inner, sys::OPUS_RESET_STATE as i32);
            Error::check(code, "opus_encoder_ctl(OPUS_RESET_STATE)")
        }
    }

    /// 1 フレーム分の PCM 音声データをエンコードする
    ///
    /// インターリーブ形式の i16 PCM データを受け取り、Opus フォーマットに圧縮する。
    /// `pcm` の長さは `frame_samples * channels` と一致する必要がある。
    pub fn encode(&mut self, pcm: &[i16]) -> Result<Vec<u8>, Error> {
        self.check_pcm_length(pcm.len(), "Encoder::encode")?;

        let size = unsafe {
            sys::opus_encode(
                self.inner,
                pcm.as_ptr(),
                self.frame_samples as c_int,
                self.encode_buf.as_mut_ptr(),
                self.encode_buf.len() as c_int,
            )
        };
        Error::check(size, "opus_encode")?;

        Ok(self.encode_buf[..size as usize].to_vec())
    }

    /// 1 フレーム分の f32 PCM 音声データをエンコードする
    ///
    /// インターリーブ形式の f32 PCM データを受け取り、Opus フォーマットに圧縮する。
    /// 入力の範囲は +/-1.0 が標準。この範囲を超えるサンプルもサポートされるが、
    /// i16 API でデコードする場合にクリップされる。
    /// `pcm` の長さは `frame_samples * channels` と一致する必要がある。
    pub fn encode_f32(&mut self, pcm: &[f32]) -> Result<Vec<u8>, Error> {
        self.check_pcm_length(pcm.len(), "Encoder::encode_f32")?;

        let size = unsafe {
            sys::opus_encode_float(
                self.inner,
                pcm.as_ptr(),
                self.frame_samples as c_int,
                self.encode_buf.as_mut_ptr(),
                self.encode_buf.len() as c_int,
            )
        };
        Error::check(size, "opus_encode_float")?;

        Ok(self.encode_buf[..size as usize].to_vec())
    }

    /// 1 フレーム分の 24bit PCM 音声データをエンコードする
    ///
    /// インターリーブ形式の i32 PCM データ (下位 24bit を使用) を受け取り、
    /// Opus フォーマットに圧縮する。
    /// `pcm` の長さは `frame_samples * channels` と一致する必要がある。
    pub fn encode_i24(&mut self, pcm: &[i32]) -> Result<Vec<u8>, Error> {
        self.check_pcm_length(pcm.len(), "Encoder::encode_i24")?;

        let size = unsafe {
            sys::opus_encode24(
                self.inner,
                pcm.as_ptr(),
                self.frame_samples as c_int,
                self.encode_buf.as_mut_ptr(),
                self.encode_buf.len() as c_int,
            )
        };
        Error::check(size, "opus_encode24")?;

        Ok(self.encode_buf[..size as usize].to_vec())
    }

    /// PCM データの長さを検証する
    fn check_pcm_length(&self, len: usize, function: &'static str) -> Result<(), Error> {
        let expected = self.frame_samples * self.channels as usize;
        if len != expected {
            return Err(Error {
                code: sys::OPUS_BAD_ARG,
                function,
            });
        }
        Ok(())
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            sys::opus_encoder_destroy(self.inner);
        }
    }
}

// Opus エンコーダー自体はスレッドセーフではないが、
// Encoder は &mut self を要求するため、同時アクセスは Rust の型システムで防がれる。
unsafe impl Send for Encoder {}

// --- デコーダー ---

/// デコーダーの設定
///
/// Opus デコーダーの生成に必要なパラメーターを保持する。
/// `Option` のフィールドは未指定時に Opus のデフォルト値が使用される。
#[derive(Debug, Clone)]
pub struct DecoderConfig {
    /// サンプルレート (Hz)
    ///
    /// Opus がサポートするサンプルレートは 8000, 12000, 16000, 24000, 48000 のいずれか。
    /// 0 を指定するとエラーが返る。
    pub sample_rate: u32,

    /// チャンネル数
    ///
    /// 1（モノラル）または 2（ステレオ）を指定する。
    /// 0 を指定するとエラーが返る。
    pub channels: u8,

    /// フレーム時間
    ///
    /// PLC (`decode_plc()`) で使用するフレームサイズを決定する。
    /// 未指定時は [`FrameDuration::Ms20`] が使用される。
    pub frame_duration: Option<FrameDuration>,

    /// ゲイン調整 (Q8 dB 単位)
    ///
    /// デコード出力を指定の Q8 dB 値でスケーリングする。
    /// 範囲: -32768 〜 32767。0 はゲイン調整なし（デフォルト）。
    /// 計算式: gain = pow(10, x / (20.0 * 256))
    /// 未指定時はゲイン調整なし。
    pub gain: Option<i32>,
}

impl DecoderConfig {
    /// 必須フィールドのみを指定してデフォルト設定を生成する
    ///
    /// オプションフィールドはすべて `None`（Opus デフォルト値）で初期化される。
    pub fn new(sample_rate: u32, channels: u8) -> Self {
        Self {
            sample_rate,
            channels,
            frame_duration: None,
            gain: None,
        }
    }
}

/// デコーダー生成中の破棄を保証するガード
///
/// `Decoder::new()` 内で CTL 設定中にエラーが発生した場合、
/// このガードが Drop 時にデコーダーを破棄する。
/// 正常完了時は `defuse()` で破棄を抑制する。
struct DecoderGuard {
    inner: *mut sys::OpusDecoder,
    defused: bool,
}

impl DecoderGuard {
    fn new(inner: *mut sys::OpusDecoder) -> Self {
        Self {
            inner,
            defused: false,
        }
    }

    fn defuse(mut self) -> *mut sys::OpusDecoder {
        self.defused = true;
        self.inner
    }
}

impl Drop for DecoderGuard {
    fn drop(&mut self) {
        if !self.defused {
            unsafe {
                sys::opus_decoder_destroy(self.inner);
            }
        }
    }
}

/// デコーダー
///
/// Opus ライブラリを使用して Opus フォーマットの音声データを PCM にデコードする。
///
/// # 使用フロー
///
/// 1. [`Decoder::new()`] でインスタンスを生成する
/// 2. [`Decoder::decode()`] で 1 パケット分の圧縮データを渡し、PCM データを受け取る
#[derive(Debug)]
pub struct Decoder {
    /// Opus デコーダーのインスタンス
    inner: *mut sys::OpusDecoder,
    /// チャンネル数
    channels: u8,
    /// 1 フレームあたりのサンプル数（チャンネルあたり）
    frame_samples: usize,
    /// デコード結果を格納するための一時バッファ
    decode_buf: Vec<i16>,
}

impl Decoder {
    /// デコーダーインスタンスを生成する
    pub fn new(config: DecoderConfig) -> Result<Self, Error> {
        if config.sample_rate == 0 {
            return Err(Error {
                code: sys::OPUS_BAD_ARG,
                function: "Decoder::new(sample_rate)",
            });
        }
        if config.channels == 0 {
            return Err(Error {
                code: sys::OPUS_BAD_ARG,
                function: "Decoder::new(channels)",
            });
        }

        let frame_duration = config.frame_duration.unwrap_or(FrameDuration::Ms20);
        let frame_samples = frame_duration.samples_per_frame(config.sample_rate);

        let mut error: c_int = 0;
        let inner = unsafe {
            sys::opus_decoder_create(
                config.sample_rate as i32,
                config.channels as c_int,
                &mut error,
            )
        };
        Error::check(error, "opus_decoder_create")?;

        // エラー時にデコーダーを確実に破棄するためのガード
        let guard = DecoderGuard::new(inner);

        // ゲイン調整設定
        if let Some(gain) = config.gain {
            unsafe {
                let code =
                    sys::opus_decoder_ctl(inner, sys::OPUS_SET_GAIN_REQUEST as i32, gain as c_int);
                Error::check(code, "opus_decoder_ctl(OPUS_SET_GAIN)")?;
            }
        }

        // すべての CTL 設定が成功したのでガードを解除する
        let inner = guard.defuse();

        Ok(Self {
            inner,
            channels: config.channels,
            frame_samples,
            decode_buf: Vec::new(),
        })
    }

    /// 1 フレームあたりのサンプル数（チャンネルあたり）を返す
    pub fn frame_samples(&self) -> usize {
        self.frame_samples
    }

    /// デコーダーの内部状態をリセットする
    ///
    /// インスタンスを再生成せずに初期状態に戻す。
    /// CTL で設定した値はそのまま保持される。
    pub fn reset(&mut self) -> Result<(), Error> {
        unsafe {
            let code = sys::opus_decoder_ctl(self.inner, sys::OPUS_RESET_STATE as i32);
            Error::check(code, "opus_decoder_ctl(OPUS_RESET_STATE)")
        }
    }

    /// 1 パケット分の圧縮データをデコードする
    ///
    /// 返される `Vec<i16>` はインターリーブ形式の PCM データ。
    pub fn decode(&mut self, encoded: &[u8]) -> Result<Vec<i16>, Error> {
        self.decode_i16_internal(encoded, 0)
    }

    /// FEC (前方誤り訂正) を使って失われたパケットを復元する
    ///
    /// パケットロス発生時に、次のパケットのデータを渡すことで
    /// 失われたフレームを FEC から復元する。
    ///
    /// エンコーダー側で `inband_fec` が有効になっている必要がある。
    /// 通常の使用フローは以下の通り:
    ///
    /// 1. パケット N のロスを検知する
    /// 2. `decode_fec(packet_n_plus_1)` で失われたフレームを復元する
    /// 3. `decode(packet_n_plus_1)` で通常のデコードを行う
    pub fn decode_fec(&mut self, encoded: &[u8]) -> Result<Vec<i16>, Error> {
        self.decode_i16_internal(encoded, 1)
    }

    /// パケットロス時に補間フレームを生成する (PLC: Packet Loss Concealment)
    ///
    /// FEC データが利用できない場合のフォールバック。
    /// デコーダーの内部状態に基づいて補間フレームを生成する。
    /// フレームサイズは [`DecoderConfig::frame_duration`] で設定した値が使用される。
    pub fn decode_plc(&mut self) -> Result<Vec<i16>, Error> {
        let buf_size = self.frame_samples * self.channels as usize;
        self.decode_buf.resize(buf_size, 0);

        let decoded_samples = unsafe {
            sys::opus_decode(
                self.inner,
                std::ptr::null(),
                0,
                self.decode_buf.as_mut_ptr(),
                self.frame_samples as c_int,
                0,
            )
        };
        Error::check(decoded_samples, "opus_decode(PLC)")?;

        let actual_size = decoded_samples as usize * self.channels as usize;
        Ok(self.decode_buf[..actual_size].to_vec())
    }

    /// 1 パケット分の圧縮データを f32 PCM にデコードする
    ///
    /// 返される `Vec<f32>` はインターリーブ形式の PCM データ。
    /// 出力の範囲は通常 +/-1.0。
    pub fn decode_f32(&mut self, encoded: &[u8]) -> Result<Vec<f32>, Error> {
        self.decode_f32_internal(encoded, 0)
    }

    /// FEC (前方誤り訂正) を使って失われたパケットを f32 PCM として復元する
    ///
    /// 動作は [`Decoder::decode_fec`] と同様。出力が f32 になる。
    pub fn decode_fec_f32(&mut self, encoded: &[u8]) -> Result<Vec<f32>, Error> {
        self.decode_f32_internal(encoded, 1)
    }

    /// パケットロス時に f32 PCM の補間フレームを生成する (PLC)
    ///
    /// 動作は [`Decoder::decode_plc`] と同様。出力が f32 になる。
    pub fn decode_plc_f32(&mut self) -> Result<Vec<f32>, Error> {
        let buf_size = self.frame_samples * self.channels as usize;
        let mut buf = vec![0.0f32; buf_size];

        let decoded_samples = unsafe {
            sys::opus_decode_float(
                self.inner,
                std::ptr::null(),
                0,
                buf.as_mut_ptr(),
                self.frame_samples as c_int,
                0,
            )
        };
        Error::check(decoded_samples, "opus_decode_float(PLC)")?;

        let actual_size = decoded_samples as usize * self.channels as usize;
        buf.truncate(actual_size);
        Ok(buf)
    }

    /// 1 パケット分の圧縮データを 24bit PCM (i32) にデコードする
    ///
    /// 返される `Vec<i32>` はインターリーブ形式の PCM データ。
    /// i32 の下位 24bit に有効なサンプル値が格納される。
    pub fn decode_i24(&mut self, encoded: &[u8]) -> Result<Vec<i32>, Error> {
        self.decode_i24_internal(encoded, 0)
    }

    /// FEC (前方誤り訂正) を使って失われたパケットを 24bit PCM (i32) として復元する
    ///
    /// 動作は [`Decoder::decode_fec`] と同様。出力が i32 (24bit) になる。
    pub fn decode_fec_i24(&mut self, encoded: &[u8]) -> Result<Vec<i32>, Error> {
        self.decode_i24_internal(encoded, 1)
    }

    /// パケットロス時に 24bit PCM (i32) の補間フレームを生成する (PLC)
    ///
    /// 動作は [`Decoder::decode_plc`] と同様。出力が i32 (24bit) になる。
    pub fn decode_plc_i24(&mut self) -> Result<Vec<i32>, Error> {
        let buf_size = self.frame_samples * self.channels as usize;
        let mut buf = vec![0i32; buf_size];

        let decoded_samples = unsafe {
            sys::opus_decode24(
                self.inner,
                std::ptr::null(),
                0,
                buf.as_mut_ptr(),
                self.frame_samples as c_int,
                0,
            )
        };
        Error::check(decoded_samples, "opus_decode24(PLC)")?;

        let actual_size = decoded_samples as usize * self.channels as usize;
        buf.truncate(actual_size);
        Ok(buf)
    }

    /// i16 デコード処理の内部実装
    fn decode_i16_internal(&mut self, encoded: &[u8], fec: c_int) -> Result<Vec<i16>, Error> {
        let nb_samples = self.get_nb_samples(encoded)?;
        let buf_size = nb_samples * self.channels as usize;
        self.decode_buf.resize(buf_size, 0);

        let decoded_samples = unsafe {
            sys::opus_decode(
                self.inner,
                encoded.as_ptr(),
                encoded.len() as c_int,
                self.decode_buf.as_mut_ptr(),
                nb_samples as c_int,
                fec,
            )
        };
        Error::check(decoded_samples, "opus_decode")?;

        let actual_size = decoded_samples as usize * self.channels as usize;
        Ok(self.decode_buf[..actual_size].to_vec())
    }

    /// f32 デコード処理の内部実装
    fn decode_f32_internal(&mut self, encoded: &[u8], fec: c_int) -> Result<Vec<f32>, Error> {
        let nb_samples = self.get_nb_samples(encoded)?;
        let buf_size = nb_samples * self.channels as usize;
        let mut buf = vec![0.0f32; buf_size];

        let decoded_samples = unsafe {
            sys::opus_decode_float(
                self.inner,
                encoded.as_ptr(),
                encoded.len() as c_int,
                buf.as_mut_ptr(),
                nb_samples as c_int,
                fec,
            )
        };
        Error::check(decoded_samples, "opus_decode_float")?;

        let actual_size = decoded_samples as usize * self.channels as usize;
        buf.truncate(actual_size);
        Ok(buf)
    }

    /// i24 デコード処理の内部実装
    fn decode_i24_internal(&mut self, encoded: &[u8], fec: c_int) -> Result<Vec<i32>, Error> {
        let nb_samples = self.get_nb_samples(encoded)?;
        let buf_size = nb_samples * self.channels as usize;
        let mut buf = vec![0i32; buf_size];

        let decoded_samples = unsafe {
            sys::opus_decode24(
                self.inner,
                encoded.as_ptr(),
                encoded.len() as c_int,
                buf.as_mut_ptr(),
                nb_samples as c_int,
                fec,
            )
        };
        Error::check(decoded_samples, "opus_decode24")?;

        let actual_size = decoded_samples as usize * self.channels as usize;
        buf.truncate(actual_size);
        Ok(buf)
    }

    /// パケットに含まれるサンプル数を取得する
    fn get_nb_samples(&self, packet: &[u8]) -> Result<usize, Error> {
        unsafe {
            let samples = sys::opus_decoder_get_nb_samples(
                self.inner,
                packet.as_ptr(),
                packet.len() as c_int,
            );
            Error::check(samples, "opus_decoder_get_nb_samples")?;
            Ok(samples as usize)
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            sys::opus_decoder_destroy(self.inner);
        }
    }
}

// Opus デコーダー自体はスレッドセーフではないが、
// Decoder は &mut self を要求するため、同時アクセスは Rust の型システムで防がれる。
unsafe impl Send for Decoder {}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SAMPLE_RATE: u32 = 48000;
    const TEST_CHANNELS: u8 = 2;
    /// 48kHz, 20ms フレームのチャンネルあたりサンプル数
    const FRAME_SAMPLES: usize = 960;
    /// 1 フレームの総サンプル数 (ステレオ)
    const FRAME_SIZE: usize = FRAME_SAMPLES * TEST_CHANNELS as usize;

    fn encoder_config(bitrate: Option<u32>) -> EncoderConfig {
        EncoderConfig {
            bitrate,
            ..EncoderConfig::new(TEST_SAMPLE_RATE, TEST_CHANNELS)
        }
    }

    fn decoder_config() -> DecoderConfig {
        DecoderConfig::new(TEST_SAMPLE_RATE, TEST_CHANNELS)
    }

    /// 440Hz サイン波のステレオ i16 PCM データを 1 フレーム分生成する
    fn sine_wave_i16() -> Vec<i16> {
        let mut pcm = vec![0i16; FRAME_SIZE];
        for i in 0..FRAME_SAMPLES {
            let t = i as f64 / TEST_SAMPLE_RATE as f64;
            let sample = (2.0 * std::f64::consts::PI * 440.0 * t).sin();
            let value = (sample * i16::MAX as f64) as i16;
            // ステレオ: 両チャンネルに同じ値
            pcm[i * 2] = value;
            pcm[i * 2 + 1] = value;
        }
        pcm
    }

    /// 440Hz サイン波のステレオ f32 PCM データを 1 フレーム分生成する
    fn sine_wave_f32() -> Vec<f32> {
        let mut pcm = vec![0.0f32; FRAME_SIZE];
        for i in 0..FRAME_SAMPLES {
            let t = i as f64 / TEST_SAMPLE_RATE as f64;
            let sample = (2.0 * std::f64::consts::PI * 440.0 * t).sin() as f32;
            pcm[i * 2] = sample;
            pcm[i * 2 + 1] = sample;
        }
        pcm
    }

    /// 440Hz サイン波のステレオ i24 (i32) PCM データを 1 フレーム分生成する
    fn sine_wave_i24() -> Vec<i32> {
        let max_24bit = 0x7F_FFFF_i32;
        let mut pcm = vec![0i32; FRAME_SIZE];
        for i in 0..FRAME_SAMPLES {
            let t = i as f64 / TEST_SAMPLE_RATE as f64;
            let sample = (2.0 * std::f64::consts::PI * 440.0 * t).sin();
            let value = (sample * max_24bit as f64) as i32;
            pcm[i * 2] = value;
            pcm[i * 2 + 1] = value;
        }
        pcm
    }

    /// i16 PCM の二乗平均平方根 (RMS) を計算する
    fn rms_i16(pcm: &[i16]) -> f64 {
        let sum: f64 = pcm.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum / pcm.len() as f64).sqrt()
    }

    /// f32 PCM の二乗平均平方根 (RMS) を計算する
    fn rms_f32(pcm: &[f32]) -> f64 {
        let sum: f64 = pcm.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum / pcm.len() as f64).sqrt()
    }

    /// i32 PCM の二乗平均平方根 (RMS) を計算する
    fn rms_i32(pcm: &[i32]) -> f64 {
        let sum: f64 = pcm.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum / pcm.len() as f64).sqrt()
    }

    // --- エンコーダー初期化テスト ---

    #[test]
    fn init_encoder() {
        // ビットレート指定あり
        assert!(Encoder::new(encoder_config(Some(64_000))).is_ok());

        // ビットレート指定なし (Opus デフォルト)
        assert!(Encoder::new(encoder_config(None)).is_ok());

        // 無効なビットレート
        assert!(Encoder::new(encoder_config(Some(0))).is_err());

        // 無効なパラメーター
        assert!(
            Encoder::new(EncoderConfig {
                sample_rate: 0,
                ..encoder_config(None)
            })
            .is_err()
        );
        assert!(
            Encoder::new(EncoderConfig {
                channels: 0,
                ..encoder_config(None)
            })
            .is_err()
        );
    }

    #[test]
    fn init_encoder_with_options() {
        let config = EncoderConfig {
            sample_rate: TEST_SAMPLE_RATE,
            channels: TEST_CHANNELS,
            bitrate: Some(128_000),
            application: Some(Application::Voip),
            frame_duration: Some(FrameDuration::Ms20),
            complexity: Some(5),
            vbr: Some(true),
            vbr_constraint: Some(true),
            max_bandwidth: Some(Bandwidth::Fullband),
            bandwidth: None,
            signal: Some(Signal::Voice),
            force_channels: None,
            inband_fec: Some(InbandFec::Enabled),
            packet_loss_perc: Some(10),
            dtx: Some(true),
            lsb_depth: Some(16),
            prediction_disabled: Some(false),
            phase_inversion_disabled: Some(false),
        };
        assert!(Encoder::new(config).is_ok());
    }

    #[test]
    fn init_encoder_cbr() {
        let config = EncoderConfig {
            vbr: Some(false),
            ..encoder_config(Some(64_000))
        };
        assert!(Encoder::new(config).is_ok());
    }

    #[test]
    fn init_encoder_low_delay() {
        let config = EncoderConfig {
            application: Some(Application::LowDelay),
            ..encoder_config(Some(64_000))
        };
        assert!(Encoder::new(config).is_ok());
    }

    #[test]
    fn init_encoder_bandwidth() {
        let config = EncoderConfig {
            bandwidth: Some(Bandwidth::Wideband),
            ..encoder_config(Some(64_000))
        };
        assert!(Encoder::new(config).is_ok());

        let config = EncoderConfig {
            max_bandwidth: Some(Bandwidth::Narrowband),
            ..encoder_config(Some(64_000))
        };
        assert!(Encoder::new(config).is_ok());
    }

    #[test]
    fn init_encoder_force_channels() {
        let config = EncoderConfig {
            force_channels: Some(ForceChannels::Mono),
            ..encoder_config(Some(64_000))
        };
        assert!(Encoder::new(config).is_ok());
    }

    #[test]
    fn init_encoder_fec() {
        let config = EncoderConfig {
            application: Some(Application::Voip),
            inband_fec: Some(InbandFec::Enabled),
            packet_loss_perc: Some(20),
            ..encoder_config(Some(64_000))
        };
        assert!(Encoder::new(config).is_ok());
    }

    // --- デコーダー初期化テスト ---

    #[test]
    fn init_decoder() {
        assert!(Decoder::new(DecoderConfig::new(TEST_SAMPLE_RATE, 2)).is_ok());
        assert!(Decoder::new(DecoderConfig::new(TEST_SAMPLE_RATE, 1)).is_ok());

        // 無効なパラメーター
        assert!(Decoder::new(DecoderConfig::new(TEST_SAMPLE_RATE, 20)).is_err());
        assert!(Decoder::new(DecoderConfig::new(TEST_SAMPLE_RATE, 0)).is_err());
        assert!(Decoder::new(DecoderConfig::new(0, 2)).is_err());
    }

    #[test]
    fn init_decoder_with_gain() {
        // ゲイン調整あり (+1 dB)
        assert!(
            Decoder::new(DecoderConfig {
                gain: Some(256),
                ..decoder_config()
            })
            .is_ok()
        );

        // 範囲外のゲイン
        assert!(
            Decoder::new(DecoderConfig {
                gain: Some(40000),
                ..decoder_config()
            })
            .is_err()
        );
    }

    // --- i16 エンコード/デコードテスト ---

    #[test]
    fn encode_pcm_length_mismatch() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        assert!(encoder.encode(&[0i16; 100]).is_err());
    }

    #[test]
    fn encode_decode_roundtrip() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_i16();
        let input_rms = rms_i16(&input);

        // エンコーダーの状態を安定させるために数フレーム捨てる
        for _ in 0..5 {
            let encoded = encoder.encode(&input).unwrap();
            decoder.decode(&encoded).unwrap();
        }

        let encoded = encoder.encode(&input).unwrap();
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(decoded.len(), FRAME_SIZE);

        // デコード結果が無音でないことを確認する
        let output_rms = rms_i16(&decoded);
        assert!(
            output_rms > input_rms * 0.5,
            "decoded RMS ({output_rms:.1}) is too low compared to input RMS ({input_rms:.1})"
        );
    }

    #[test]
    fn decode_fec() {
        let config = EncoderConfig {
            application: Some(Application::Voip),
            inband_fec: Some(InbandFec::Enabled),
            packet_loss_perc: Some(50),
            ..encoder_config(Some(64_000))
        };
        let mut encoder = Encoder::new(config).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_i16();

        // エンコーダーの状態を安定させる
        for _ in 0..5 {
            let encoded = encoder.encode(&input).unwrap();
            decoder.decode(&encoded).unwrap();
        }

        let packet1 = encoder.encode(&input).unwrap();
        let packet2 = encoder.encode(&input).unwrap();

        // packet1 を通常デコードする
        decoder.decode(&packet1).unwrap();

        // packet2 の FEC で packet1 相当のフレームを復元する
        let fec_decoded = decoder.decode_fec(&packet2).unwrap();
        assert_eq!(fec_decoded.len(), FRAME_SIZE);

        // FEC 復元結果が無音でないことを確認する
        let fec_rms = rms_i16(&fec_decoded);
        assert!(
            fec_rms > 0.0,
            "FEC decoded frame should not be silent, got RMS={fec_rms}"
        );

        // packet2 を通常デコードする
        let decoded2 = decoder.decode(&packet2).unwrap();
        let output_rms = rms_i16(&decoded2);
        assert!(
            output_rms > 0.0,
            "decoded frame after FEC should not be silent"
        );
    }

    #[test]
    fn decode_plc() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_i16();

        // エンコーダーの状態を安定させる
        for _ in 0..5 {
            let encoded = encoder.encode(&input).unwrap();
            decoder.decode(&encoded).unwrap();
        }

        // 最後のフレームをデコードしてデコーダーに状態を持たせる
        let encoded = encoder.encode(&input).unwrap();
        decoder.decode(&encoded).unwrap();

        // PLC で補間フレームを生成する
        let plc = decoder.decode_plc().unwrap();
        assert_eq!(plc.len(), FRAME_SIZE);

        // サイン波入力後の PLC は無音ではないはず
        let plc_rms = rms_i16(&plc);
        assert!(
            plc_rms > 0.0,
            "PLC frame after sine wave should not be silent, got RMS={plc_rms}"
        );
    }

    // --- f32 エンコード/デコードテスト ---

    #[test]
    fn encode_f32_pcm_length_mismatch() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        assert!(encoder.encode_f32(&[0.0f32; 100]).is_err());
    }

    #[test]
    fn encode_f32_decode_f32_roundtrip() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_f32();
        let input_rms = rms_f32(&input);

        // エンコーダーの状態を安定させる
        for _ in 0..5 {
            let encoded = encoder.encode_f32(&input).unwrap();
            decoder.decode_f32(&encoded).unwrap();
        }

        let encoded = encoder.encode_f32(&input).unwrap();
        let decoded = decoder.decode_f32(&encoded).unwrap();

        assert_eq!(decoded.len(), FRAME_SIZE);

        // デコード結果の RMS が入力の 50% 以上であることを確認する
        let output_rms = rms_f32(&decoded);
        assert!(
            output_rms > input_rms * 0.5,
            "decoded RMS ({output_rms:.4}) is too low compared to input RMS ({input_rms:.4})"
        );
    }

    #[test]
    fn decode_fec_f32() {
        let config = EncoderConfig {
            application: Some(Application::Voip),
            inband_fec: Some(InbandFec::Enabled),
            packet_loss_perc: Some(50),
            ..encoder_config(Some(64_000))
        };
        let mut encoder = Encoder::new(config).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_f32();

        // エンコーダーの状態を安定させる
        for _ in 0..5 {
            let encoded = encoder.encode_f32(&input).unwrap();
            decoder.decode_f32(&encoded).unwrap();
        }

        let packet1 = encoder.encode_f32(&input).unwrap();
        let packet2 = encoder.encode_f32(&input).unwrap();

        decoder.decode_f32(&packet1).unwrap();

        let fec_decoded = decoder.decode_fec_f32(&packet2).unwrap();
        assert_eq!(fec_decoded.len(), FRAME_SIZE);

        let fec_rms = rms_f32(&fec_decoded);
        assert!(
            fec_rms > 0.0,
            "FEC decoded f32 frame should not be silent, got RMS={fec_rms}"
        );

        let decoded2 = decoder.decode_f32(&packet2).unwrap();
        let output_rms = rms_f32(&decoded2);
        assert!(
            output_rms > 0.0,
            "decoded f32 frame after FEC should not be silent"
        );
    }

    #[test]
    fn decode_plc_f32() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_f32();

        for _ in 0..5 {
            let encoded = encoder.encode_f32(&input).unwrap();
            decoder.decode_f32(&encoded).unwrap();
        }

        let encoded = encoder.encode_f32(&input).unwrap();
        decoder.decode_f32(&encoded).unwrap();

        let plc = decoder.decode_plc_f32().unwrap();
        assert_eq!(plc.len(), FRAME_SIZE);

        let plc_rms = rms_f32(&plc);
        assert!(
            plc_rms > 0.0,
            "PLC f32 frame after sine wave should not be silent, got RMS={plc_rms}"
        );
    }

    // --- i24 エンコード/デコードテスト ---

    #[test]
    fn encode_i24_pcm_length_mismatch() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        assert!(encoder.encode_i24(&[0i32; 100]).is_err());
    }

    #[test]
    fn encode_i24_decode_i24_roundtrip() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_i24();
        let input_rms = rms_i32(&input);

        // エンコーダーの状態を安定させる
        for _ in 0..5 {
            let encoded = encoder.encode_i24(&input).unwrap();
            decoder.decode_i24(&encoded).unwrap();
        }

        let encoded = encoder.encode_i24(&input).unwrap();
        let decoded = decoder.decode_i24(&encoded).unwrap();

        assert_eq!(decoded.len(), FRAME_SIZE);

        // デコード結果の RMS が入力の 50% 以上であることを確認する
        let output_rms = rms_i32(&decoded);
        assert!(
            output_rms > input_rms * 0.5,
            "decoded i24 RMS ({output_rms:.1}) is too low compared to input RMS ({input_rms:.1})"
        );
    }

    #[test]
    fn decode_fec_i24() {
        let config = EncoderConfig {
            application: Some(Application::Voip),
            inband_fec: Some(InbandFec::Enabled),
            packet_loss_perc: Some(50),
            ..encoder_config(Some(64_000))
        };
        let mut encoder = Encoder::new(config).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_i24();

        for _ in 0..5 {
            let encoded = encoder.encode_i24(&input).unwrap();
            decoder.decode_i24(&encoded).unwrap();
        }

        let packet1 = encoder.encode_i24(&input).unwrap();
        let packet2 = encoder.encode_i24(&input).unwrap();

        decoder.decode_i24(&packet1).unwrap();

        let fec_decoded = decoder.decode_fec_i24(&packet2).unwrap();
        assert_eq!(fec_decoded.len(), FRAME_SIZE);

        let fec_rms = rms_i32(&fec_decoded);
        assert!(
            fec_rms > 0.0,
            "FEC decoded i24 frame should not be silent, got RMS={fec_rms}"
        );

        let decoded2 = decoder.decode_i24(&packet2).unwrap();
        let output_rms = rms_i32(&decoded2);
        assert!(
            output_rms > 0.0,
            "decoded i24 frame after FEC should not be silent"
        );
    }

    #[test]
    fn decode_plc_i24() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_i24();

        for _ in 0..5 {
            let encoded = encoder.encode_i24(&input).unwrap();
            decoder.decode_i24(&encoded).unwrap();
        }

        let encoded = encoder.encode_i24(&input).unwrap();
        decoder.decode_i24(&encoded).unwrap();

        let plc = decoder.decode_plc_i24().unwrap();
        assert_eq!(plc.len(), FRAME_SIZE);

        let plc_rms = rms_i32(&plc);
        assert!(
            plc_rms > 0.0,
            "PLC i24 frame after sine wave should not be silent, got RMS={plc_rms}"
        );
    }

    // --- リセットテスト ---

    #[test]
    fn encoder_reset() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();

        let input = sine_wave_i16();
        encoder.encode(&input).unwrap();
        encoder.reset().unwrap();

        // リセット後もサイン波を正常にエンコードできる
        let encoded = encoder.encode(&input).unwrap();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn decoder_reset() {
        let mut encoder = Encoder::new(encoder_config(Some(64_000))).unwrap();
        let mut decoder = Decoder::new(decoder_config()).unwrap();

        let input = sine_wave_i16();
        let input_rms = rms_i16(&input);

        // エンコーダーの状態を安定させる
        for _ in 0..5 {
            let encoded = encoder.encode(&input).unwrap();
            decoder.decode(&encoded).unwrap();
        }

        decoder.reset().unwrap();

        // リセット後、再度安定させてからデコード結果を検証する
        for _ in 0..5 {
            let encoded = encoder.encode(&input).unwrap();
            decoder.decode(&encoded).unwrap();
        }

        let encoded = encoder.encode(&input).unwrap();
        let decoded = decoder.decode(&encoded).unwrap();
        let output_rms = rms_i16(&decoded);
        assert!(
            output_rms > input_rms * 0.5,
            "decoded RMS after reset ({output_rms:.1}) is too low"
        );
    }

    // --- その他のテスト ---

    #[test]
    fn error_reason() {
        let e = Error::check(sys::OPUS_BAD_ARG, "test").expect_err("not an error");
        assert!(e.reason().is_some());
    }

    #[test]
    fn opus_version_string() {
        let version = version_string();
        assert!(version.starts_with("libopus"));
    }
}
