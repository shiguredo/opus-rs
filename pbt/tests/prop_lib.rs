//! エンコーダー / デコーダーとパケットユーティリティ関数の PBT
//!
//! `src/lib.rs` (クレートルート) の公開 API を対象とする。
//! 任意入力に対するクラッシュ耐性 (パニック安全性) は `fuzz/` が担当するため、
//! ここではラウンドトリップなどのプロパティ検証のみを行う。

use proptest::prelude::*;
use shiguredo_opus::{
    Decoder, DecoderConfig, Encoder, EncoderConfig, FrameDuration, packet_get_bandwidth,
    packet_get_nb_channels, packet_get_nb_frames, packet_get_nb_samples,
    packet_get_samples_per_frame,
};

/// ランダムなフレーム時間を生成する
fn arb_frame_duration() -> impl Strategy<Value = FrameDuration> {
    prop_oneof![
        Just(FrameDuration::Ms2_5),
        Just(FrameDuration::Ms5),
        Just(FrameDuration::Ms10),
        Just(FrameDuration::Ms20),
        Just(FrameDuration::Ms40),
        Just(FrameDuration::Ms60),
    ]
}

/// ランダムなエンコーダー設定を生成する
///
/// (サンプルレート, チャンネル数, フレーム時間, ビットレート) の組を返す。
/// Opus がサポートするサンプルレートは 8000 / 12000 / 16000 / 24000 / 48000 Hz のいずれか。
fn arb_config() -> impl Strategy<Value = (u32, u8, FrameDuration, u32)> {
    (
        prop_oneof![
            Just(8000u32),
            Just(12000),
            Just(16000),
            Just(24000),
            Just(48000),
        ],
        prop_oneof![Just(1u8), Just(2)],
        arb_frame_duration(),
        6000..=256_000u32,
    )
}

/// フレーム時間とサンプルレートから 1 フレームあたりのサンプル数（チャンネルあたり）を算出する
///
/// 入力生成に使うためにここに定義する。
/// 実装とのズレは、各テストで公開 API の [`Encoder::frame_samples`] /
/// [`Decoder::frame_samples`] と一致することを検証して防ぐ。
fn frame_samples_of(sample_rate: u32, frame_duration: FrameDuration) -> usize {
    match frame_duration {
        FrameDuration::Ms2_5 => sample_rate as usize / 400,
        FrameDuration::Ms5 => sample_rate as usize / 200,
        FrameDuration::Ms10 => sample_rate as usize / 100,
        FrameDuration::Ms20 => sample_rate as usize / 50,
        FrameDuration::Ms40 => sample_rate as usize / 25,
        FrameDuration::Ms60 => sample_rate as usize * 3 / 50,
    }
}

/// i16 ラウンドトリップの入力を生成する
///
/// (サンプルレート, チャンネル数, フレーム時間, ビットレート, PCM) の組を返す。
/// PCM の長さは設定から導出される 1 フレーム分の総サンプル数に一致させる。
fn arb_roundtrip_i16() -> impl Strategy<Value = (u32, u8, FrameDuration, u32, Vec<i16>)> {
    arb_config().prop_flat_map(|(sample_rate, channels, frame_duration, bitrate)| {
        let total = frame_samples_of(sample_rate, frame_duration) * channels as usize;
        (
            Just(sample_rate),
            Just(channels),
            Just(frame_duration),
            Just(bitrate),
            prop::collection::vec(any::<i16>(), total),
        )
    })
}

/// f32 ラウンドトリップの入力を生成する
///
/// f32 API は +/-1.0 を標準入力とする。ここでは標準範囲で生成する。
fn arb_roundtrip_f32() -> impl Strategy<Value = (u32, u8, FrameDuration, u32, Vec<f32>)> {
    arb_config().prop_flat_map(|(sample_rate, channels, frame_duration, bitrate)| {
        let total = frame_samples_of(sample_rate, frame_duration) * channels as usize;
        (
            Just(sample_rate),
            Just(channels),
            Just(frame_duration),
            Just(bitrate),
            prop::collection::vec(-1.0f32..=1.0, total),
        )
    })
}

/// i24 (i32) ラウンドトリップの入力を生成する
///
/// i24 API は i32 の下位 24bit を使用するため、-2^23 以上 2^23-1 以下の範囲で生成する。
fn arb_roundtrip_i24() -> impl Strategy<Value = (u32, u8, FrameDuration, u32, Vec<i32>)> {
    arb_config().prop_flat_map(|(sample_rate, channels, frame_duration, bitrate)| {
        let total = frame_samples_of(sample_rate, frame_duration) * channels as usize;
        (
            Just(sample_rate),
            Just(channels),
            Just(frame_duration),
            Just(bitrate),
            prop::collection::vec(-0x80_0000i32..=0x7F_FFFF, total),
        )
    })
}

/// ラウンドトリップに使うエンコーダーとデコーダーを生成する
///
/// 有効な設定で生成するため、失敗するのは実装バグ。
fn roundtrip_encoders(
    sample_rate: u32,
    channels: u8,
    frame_duration: FrameDuration,
    bitrate: u32,
) -> (Encoder, Decoder) {
    let enc_config = EncoderConfig {
        bitrate: Some(bitrate),
        frame_duration: Some(frame_duration),
        ..EncoderConfig::new(sample_rate, channels)
    };
    let dec_config = DecoderConfig {
        sample_rate,
        channels,
        frame_duration: Some(frame_duration),
        gain: None,
    };
    let encoder = Encoder::new(enc_config)
        .expect("有効な設定なのでエンコーダーの生成は成功するはず");
    let decoder = Decoder::new(dec_config)
        .expect("有効な設定なのでデコーダーの生成は成功するはず");
    (encoder, decoder)
}

proptest! {
    /// i16 PCM のエンコード / デコードがラウンドトリップで成功する
    ///
    /// - 入力生成に使った式と公開 API のフレームサンプル数が一致する
    /// - エンコード結果が空パケットにならない
    /// - デコード結果の長さが 1 フレーム分 (frame_samples × channels) と一致する
    #[test]
    fn encode_decode_roundtrip_i16(
        (sample_rate, channels, frame_duration, bitrate, pcm) in arb_roundtrip_i16()
    ) {
        let (mut encoder, mut decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        // 入力生成に使った式と公開 API のフレームサンプル数が一致することを確認する
        prop_assert_eq!(
            encoder.frame_samples(),
            frame_samples_of(sample_rate, frame_duration),
            "frame_samples の算出が実装とズレている"
        );

        let encoded = encoder.encode(&pcm)
            .expect("有効な PCM 入力なのでエンコードは成功するはず");
        prop_assert!(!encoded.is_empty(), "エンコード結果が空パケットになっている");

        let decoded = decoder.decode(&encoded)
            .expect("エンコード済みのパケットなのでデコードは成功するはず");
        prop_assert_eq!(
            decoded.len(),
            frame_samples_of(sample_rate, frame_duration) * channels as usize,
            "デコード結果の長さが 1 フレーム分と一致しない"
        );
    }

    /// f32 PCM のエンコード / デコードがラウンドトリップで成功する
    ///
    /// i16 版に加えて、デコード結果に非有限値 (NaN / Inf) が含まれないことを確認する。
    #[test]
    fn encode_decode_roundtrip_f32(
        (sample_rate, channels, frame_duration, bitrate, pcm) in arb_roundtrip_f32()
    ) {
        let (mut encoder, mut decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        prop_assert_eq!(
            encoder.frame_samples(),
            frame_samples_of(sample_rate, frame_duration),
            "frame_samples の算出が実装とズレている"
        );

        let encoded = encoder.encode_f32(&pcm)
            .expect("有効な PCM 入力なのでエンコードは成功するはず");
        prop_assert!(!encoded.is_empty(), "エンコード結果が空パケットになっている");

        let decoded = decoder.decode_f32(&encoded)
            .expect("エンコード済みのパケットなのでデコードは成功するはず");
        prop_assert_eq!(
            decoded.len(),
            frame_samples_of(sample_rate, frame_duration) * channels as usize,
            "デコード結果の長さが 1 フレーム分と一致しない"
        );
        prop_assert!(
            decoded.iter().all(|s| s.is_finite()),
            "f32 デコード結果に非有限値 (NaN / Inf) が含まれる"
        );
    }

    /// i24 PCM (i32) のエンコード / デコードがラウンドトリップで成功する
    #[test]
    fn encode_decode_roundtrip_i24(
        (sample_rate, channels, frame_duration, bitrate, pcm) in arb_roundtrip_i24()
    ) {
        let (mut encoder, mut decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        prop_assert_eq!(
            encoder.frame_samples(),
            frame_samples_of(sample_rate, frame_duration),
            "frame_samples の算出が実装とズレている"
        );

        let encoded = encoder.encode_i24(&pcm)
            .expect("有効な PCM 入力なのでエンコードは成功するはず");
        prop_assert!(!encoded.is_empty(), "エンコード結果が空パケットになっている");

        let decoded = decoder.decode_i24(&encoded)
            .expect("エンコード済みのパケットなのでデコードは成功するはず");
        prop_assert_eq!(
            decoded.len(),
            frame_samples_of(sample_rate, frame_duration) * channels as usize,
            "デコード結果の長さが 1 フレーム分と一致しない"
        );
    }

    /// エンコード済みパケットのサンプル数情報が一貫している
    ///
    /// パケット全体のサンプル数は 1 フレームあたりのサンプル数 × フレーム数に等しい。
    #[test]
    fn packet_info_consistent(
        (sample_rate, channels, frame_duration, bitrate, pcm) in arb_roundtrip_i16()
    ) {
        let (mut encoder, _decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        let encoded = encoder.encode(&pcm)
            .expect("有効な PCM 入力なのでエンコードは成功するはず");

        // エンコード済みパケットなのでパースは必ず成功する
        let nb_frames = packet_get_nb_frames(&encoded)
            .expect("エンコード済みのパケットなのでフレーム数の取得は成功するはず");
        let samples_per_frame = packet_get_samples_per_frame(&encoded, sample_rate)
            .expect("エンコード済みのパケットなのでフレームサンプル数の取得は成功するはず");
        let nb_samples = packet_get_nb_samples(&encoded, sample_rate)
            .expect("エンコード済みのパケットなので総サンプル数の取得は成功するはず");
        let nb_packet_channels = packet_get_nb_channels(&encoded)
            .expect("エンコード済みのパケットなのでチャンネル数の取得は成功するはず");

        // フレーム数は少なくとも 1
        prop_assert!(nb_frames >= 1, "フレーム数が 0 になっている");

        // パケット全体のサンプル数は 1 フレームあたりのサンプル数 × フレーム数と一致する
        prop_assert_eq!(
            nb_samples,
            samples_per_frame * nb_frames,
            "パケットのサンプル数情報が一貫していない"
        );

        // チャンネル数は 1 か 2
        prop_assert!(
            nb_packet_channels == 1 || nb_packet_channels == 2,
            "想定外のチャンネル数: {nb_packet_channels}"
        );

        // 帯域幅は必ず取得できる (エンコード済みパケットならエラーにならない)
        prop_assert!(
            packet_get_bandwidth(&encoded).is_ok(),
            "エンコード済みパケットの帯域幅が取得できない"
        );
    }

    /// どの設定でも PLC デコードが成功し、1 フレーム分の長さが返る
    ///
    /// PLC はデコーダーの内部状態に基づいて補間フレームを生成するため、
    /// エンコード済みパケットを一度デコードしてから呼び出す。
    #[test]
    fn decode_plc_length(
        (sample_rate, channels, frame_duration, bitrate, pcm) in arb_roundtrip_i16()
    ) {
        let (mut encoder, mut decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        // デコーダーに内部状態を持たせる
        let encoded = encoder.encode(&pcm)
            .expect("有効な PCM 入力なのでエンコードは成功するはず");
        decoder.decode(&encoded)
            .expect("エンコード済みのパケットなのでデコードは成功するはず");

        let plc = decoder.decode_plc()
            .expect("デコーダーに状態があるので PLC デコードは成功するはず");
        prop_assert_eq!(
            plc.len(),
            frame_samples_of(sample_rate, frame_duration) * channels as usize,
            "PLC デコード結果の長さが 1 フレーム分と一致しない"
        );
    }
}
