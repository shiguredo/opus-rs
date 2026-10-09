//! エンコーダー / デコーダーとパケットユーティリティ関数の PBT
//!
//! `src/lib.rs` (クレートルート) の公開 API を対象とする。
//! 任意入力に対するクラッシュ耐性 (パニック安全性) は `fuzz/` が担当するため、
//! ここではラウンドトリップなどのプロパティ検証のみを行う。

use std::cell::Cell;

use noprop::{TestCaseContext, TestResult};
use shiguredo_opus::{
    Decoder, DecoderConfig, Encoder, EncoderConfig, FrameDuration, packet_get_bandwidth,
    packet_get_nb_channels, packet_get_nb_frames, packet_get_nb_samples,
    packet_get_samples_per_frame,
};

/// PBT の 1 テストあたりのケース数
///
/// 設定の組み合わせは 5 サンプルレート × 2 チャンネル × 6 フレーム時間 = 60 通りで、
/// 256 ケースあれば各組み合わせを複数回探索できる。
const CASES: usize = 256;

/// Opus がサポートするサンプルレート (Hz)
const SAMPLE_RATES: &[u32] = &[8000, 12000, 16000, 24000, 48000];

/// 全フレーム時間
const FRAME_DURATIONS: &[FrameDuration] = &[
    FrameDuration::Ms2_5,
    FrameDuration::Ms5,
    FrameDuration::Ms10,
    FrameDuration::Ms20,
    FrameDuration::Ms40,
    FrameDuration::Ms60,
];

/// ビットレートの生成範囲 (bps)
///
/// Opus が有効とする 500〜512000 bps の内側を探索する。
const BITRATE_RANGE: std::ops::RangeInclusive<u32> = 6000..=256_000;

/// 境界値に割り当てる確率
///
/// 最小値と最大値に 1/4、内側の値に 3/4 を割り当てる。
const BOUNDARY_RATIO: noprop::Ratio = noprop::Ratio::one_nth(4);

/// エンコーダー / デコーダー設定を構成するパラメーター
///
/// (サンプルレート, チャンネル数, フレーム時間, ビットレート) の組。
type Config = (u32, u8, FrameDuration, u32);

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

/// サンプルレートを境界値込みで生成する
fn sample_sample_rate(ctx: &mut TestCaseContext) -> u32 {
    noprop::sample_with_boundaries(ctx, &[8000, 48000], BOUNDARY_RATIO, |ctx| {
        noprop::sample_choice(ctx, SAMPLE_RATES)
    })
}

/// フレーム時間を境界値込みで生成する
fn sample_frame_duration(ctx: &mut TestCaseContext) -> FrameDuration {
    noprop::sample_with_boundaries(
        ctx,
        &[FrameDuration::Ms2_5, FrameDuration::Ms60],
        BOUNDARY_RATIO,
        |ctx| noprop::sample_choice(ctx, FRAME_DURATIONS),
    )
}

/// チャンネル数を生成する
fn sample_channels(ctx: &mut TestCaseContext) -> u8 {
    noprop::sample_choice(ctx, &[1, 2])
}

/// ビットレートを境界値込みで生成する
fn sample_bitrate(ctx: &mut TestCaseContext) -> u32 {
    noprop::sample_with_boundaries(ctx, &[6000, 256_000], BOUNDARY_RATIO, |ctx| {
        noprop::sample_u32_in(ctx, BITRATE_RANGE)
    })
}

/// エンコーダー / デコーダー設定のパラメーターを生成する
fn sample_config(ctx: &mut TestCaseContext) -> Config {
    (
        sample_sample_rate(ctx),
        sample_channels(ctx),
        sample_frame_duration(ctx),
        sample_bitrate(ctx),
    )
}

/// i16 PCM を指定サンプル数だけ生成する
fn sample_pcm_i16(ctx: &mut TestCaseContext, len: usize) -> Vec<i16> {
    let mut pcm = Vec::new();
    for _ in 0..len {
        pcm.push(noprop::sample_i16(ctx));
    }
    pcm
}

/// f32 PCM を指定サンプル数だけ生成する
///
/// f32 API は +/-1.0 を標準入力とする。ここでは標準範囲で生成する。
fn sample_pcm_f32(ctx: &mut TestCaseContext, len: usize) -> Vec<f32> {
    let mut pcm = Vec::new();
    for _ in 0..len {
        pcm.push(noprop::sample_f32_in(ctx, -1.0, 1.0));
    }
    pcm
}

/// i24 PCM (i32) を指定サンプル数だけ生成する
///
/// i24 API は i32 の下位 24bit を使用するため、-2^23 以上 2^23-1 以下の範囲で生成する。
fn sample_pcm_i24(ctx: &mut TestCaseContext, len: usize) -> Vec<i32> {
    let mut pcm = Vec::new();
    for _ in 0..len {
        pcm.push(noprop::sample_i32_in(ctx, -0x80_0000..=0x7F_FFFF));
    }
    pcm
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
    let encoder =
        Encoder::new(enc_config).expect("有効な設定なのでエンコーダーの生成は成功するはず");
    let decoder = Decoder::new(dec_config).expect("有効な設定なのでデコーダーの生成は成功するはず");
    (encoder, decoder)
}

/// i16 PCM のエンコード / デコードがラウンドトリップで成功する
///
/// - 入力生成に使った式と公開 API のフレームサンプル数が一致する
/// - エンコード結果が空パケットにならない
/// - デコード結果の長さが 1 フレーム分 (frame_samples × channels) と一致する
#[test]
fn encode_decode_roundtrip_i16() -> TestResult {
    let seed = noprop::seed_from_env_or_time("SHIGUREDO_OPUS_PBT_SEED")?;
    // モノラルとステレオの両方のケースが実行されたことを実行後に検証するためのゲート。
    // 片方しか生成しないサンプラーのバグで検証が空振りするのを防ぐ。
    let mono_cases = Cell::new(0usize);
    let stereo_cases = Cell::new(0usize);
    let mut runner = noprop::Runner::new(seed);

    runner.run(CASES, |ctx| {
        let (sample_rate, channels, frame_duration, bitrate) = sample_config(ctx);
        let frame_samples = frame_samples_of(sample_rate, frame_duration);
        let pcm = sample_pcm_i16(ctx, frame_samples * channels as usize);
        let (mut encoder, mut decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        // 入力生成に使った式と公開 API のフレームサンプル数が一致することを確認する
        assert_eq!(
            encoder.frame_samples(),
            frame_samples,
            "frame_samples の算出が実装とズレている"
        );

        let encoded = encoder
            .encode(&pcm)
            .expect("有効な PCM 入力なのでエンコードは成功するはず");
        assert!(
            !encoded.is_empty(),
            "エンコード結果が空パケットになっている"
        );

        let decoded = decoder
            .decode(&encoded)
            .expect("エンコード済みのパケットなのでデコードは成功するはず");
        assert_eq!(
            decoded.len(),
            frame_samples * channels as usize,
            "デコード結果の長さが 1 フレーム分と一致しない"
        );

        // 不変条件を検証し終えた地点でゲートを更新する
        if channels == 1 {
            mono_cases.set(mono_cases.get() + 1);
        } else {
            stereo_cases.set(stereo_cases.get() + 1);
        }
        Ok(())
    })?;

    assert!(
        mono_cases.get() > 0,
        "モノラルのケースが 1 件も無い\n{runner}"
    );
    assert!(
        stereo_cases.get() > 0,
        "ステレオのケースが 1 件も無い\n{runner}"
    );
    Ok(())
}

/// f32 PCM のエンコード / デコードがラウンドトリップで成功する
///
/// i16 版に加えて、デコード結果に非有限値 (NaN / Inf) が含まれないことを確認する。
#[test]
fn encode_decode_roundtrip_f32() -> TestResult {
    let seed = noprop::seed_from_env_or_time("SHIGUREDO_OPUS_PBT_SEED")?;
    // モノラルとステレオの両方のケースが実行されたことを実行後に検証するためのゲート
    let mono_cases = Cell::new(0usize);
    let stereo_cases = Cell::new(0usize);
    let mut runner = noprop::Runner::new(seed);

    runner.run(CASES, |ctx| {
        let (sample_rate, channels, frame_duration, bitrate) = sample_config(ctx);
        let frame_samples = frame_samples_of(sample_rate, frame_duration);
        let pcm = sample_pcm_f32(ctx, frame_samples * channels as usize);
        let (mut encoder, mut decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        assert_eq!(
            encoder.frame_samples(),
            frame_samples,
            "frame_samples の算出が実装とズレている"
        );

        let encoded = encoder
            .encode_f32(&pcm)
            .expect("有効な PCM 入力なのでエンコードは成功するはず");
        assert!(
            !encoded.is_empty(),
            "エンコード結果が空パケットになっている"
        );

        let decoded = decoder
            .decode_f32(&encoded)
            .expect("エンコード済みのパケットなのでデコードは成功するはず");
        assert_eq!(
            decoded.len(),
            frame_samples * channels as usize,
            "デコード結果の長さが 1 フレーム分と一致しない"
        );
        assert!(
            decoded.iter().all(|s| s.is_finite()),
            "f32 デコード結果に非有限値 (NaN / Inf) が含まれる"
        );

        // 不変条件を検証し終えた地点でゲートを更新する
        if channels == 1 {
            mono_cases.set(mono_cases.get() + 1);
        } else {
            stereo_cases.set(stereo_cases.get() + 1);
        }
        Ok(())
    })?;

    assert!(
        mono_cases.get() > 0,
        "モノラルのケースが 1 件も無い\n{runner}"
    );
    assert!(
        stereo_cases.get() > 0,
        "ステレオのケースが 1 件も無い\n{runner}"
    );
    Ok(())
}

/// i24 PCM (i32) のエンコード / デコードがラウンドトリップで成功する
#[test]
fn encode_decode_roundtrip_i24() -> TestResult {
    let seed = noprop::seed_from_env_or_time("SHIGUREDO_OPUS_PBT_SEED")?;
    // モノラルとステレオの両方のケースが実行されたことを実行後に検証するためのゲート
    let mono_cases = Cell::new(0usize);
    let stereo_cases = Cell::new(0usize);
    let mut runner = noprop::Runner::new(seed);

    runner.run(CASES, |ctx| {
        let (sample_rate, channels, frame_duration, bitrate) = sample_config(ctx);
        let frame_samples = frame_samples_of(sample_rate, frame_duration);
        let pcm = sample_pcm_i24(ctx, frame_samples * channels as usize);
        let (mut encoder, mut decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        assert_eq!(
            encoder.frame_samples(),
            frame_samples,
            "frame_samples の算出が実装とズレている"
        );

        let encoded = encoder
            .encode_i24(&pcm)
            .expect("有効な PCM 入力なのでエンコードは成功するはず");
        assert!(
            !encoded.is_empty(),
            "エンコード結果が空パケットになっている"
        );

        let decoded = decoder
            .decode_i24(&encoded)
            .expect("エンコード済みのパケットなのでデコードは成功するはず");
        assert_eq!(
            decoded.len(),
            frame_samples * channels as usize,
            "デコード結果の長さが 1 フレーム分と一致しない"
        );

        // 不変条件を検証し終えた地点でゲートを更新する
        if channels == 1 {
            mono_cases.set(mono_cases.get() + 1);
        } else {
            stereo_cases.set(stereo_cases.get() + 1);
        }
        Ok(())
    })?;

    assert!(
        mono_cases.get() > 0,
        "モノラルのケースが 1 件も無い\n{runner}"
    );
    assert!(
        stereo_cases.get() > 0,
        "ステレオのケースが 1 件も無い\n{runner}"
    );
    Ok(())
}

/// エンコード済みパケットのサンプル数情報が一貫している
///
/// パケット全体のサンプル数は 1 フレームあたりのサンプル数 × フレーム数に等しい。
#[test]
fn packet_info_consistent() -> TestResult {
    let seed = noprop::seed_from_env_or_time("SHIGUREDO_OPUS_PBT_SEED")?;
    let mut runner = noprop::Runner::new(seed);

    runner.run(CASES, |ctx| {
        let (sample_rate, channels, frame_duration, bitrate) = sample_config(ctx);
        let frame_samples = frame_samples_of(sample_rate, frame_duration);
        let pcm = sample_pcm_i16(ctx, frame_samples * channels as usize);
        let (mut encoder, _decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        let encoded = encoder
            .encode(&pcm)
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
        assert!(nb_frames >= 1, "フレーム数が 0 になっている");

        // パケット全体のサンプル数は 1 フレームあたりのサンプル数 × フレーム数と一致する
        assert_eq!(
            nb_samples,
            samples_per_frame * nb_frames,
            "パケットのサンプル数情報が一貫していない"
        );

        // チャンネル数は 1 か 2
        assert!(
            nb_packet_channels == 1 || nb_packet_channels == 2,
            "想定外のチャンネル数: {nb_packet_channels}"
        );

        // 帯域幅は必ず取得できる (エンコード済みパケットならエラーにならない)
        assert!(
            packet_get_bandwidth(&encoded).is_ok(),
            "エンコード済みパケットの帯域幅が取得できない"
        );
        Ok(())
    })?;
    Ok(())
}

/// どの設定でも PLC デコードが成功し、1 フレーム分の長さが返る
///
/// PLC はデコーダーの内部状態に基づいて補間フレームを生成するため、
/// エンコード済みパケットを一度デコードしてから呼び出す。
#[test]
fn decode_plc_length() -> TestResult {
    let seed = noprop::seed_from_env_or_time("SHIGUREDO_OPUS_PBT_SEED")?;
    let mut runner = noprop::Runner::new(seed);

    runner.run(CASES, |ctx| {
        let (sample_rate, channels, frame_duration, bitrate) = sample_config(ctx);
        let frame_samples = frame_samples_of(sample_rate, frame_duration);
        let pcm = sample_pcm_i16(ctx, frame_samples * channels as usize);
        let (mut encoder, mut decoder) =
            roundtrip_encoders(sample_rate, channels, frame_duration, bitrate);

        // デコーダーに内部状態を持たせる
        let encoded = encoder
            .encode(&pcm)
            .expect("有効な PCM 入力なのでエンコードは成功するはず");
        decoder
            .decode(&encoded)
            .expect("エンコード済みのパケットなのでデコードは成功するはず");

        let plc = decoder
            .decode_plc()
            .expect("デコーダーに状態があるので PLC デコードは成功するはず");
        assert_eq!(
            plc.len(),
            frame_samples * channels as usize,
            "PLC デコード結果の長さが 1 フレーム分と一致しない"
        );
        Ok(())
    })?;
    Ok(())
}
