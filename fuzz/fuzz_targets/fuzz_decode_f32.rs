#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use shiguredo_opus::{DecoderConfig, FrameDuration};

/// ファジング入力からデコーダー設定を決定するための構造体
#[derive(Arbitrary, Debug)]
struct FuzzInput {
    /// サンプルレート選択
    sample_rate_index: u8,
    /// チャンネル数
    stereo: bool,
    /// フレーム時間選択
    frame_duration_index: u8,
    /// デコード対象のバイト列
    data: Vec<u8>,
}

/// インデックスからサンプルレートを取得する
fn sample_rate_from_index(index: u8) -> u32 {
    match index % 5 {
        0 => 8000,
        1 => 12000,
        2 => 16000,
        3 => 24000,
        _ => 48000,
    }
}

/// インデックスから FrameDuration を取得する
fn frame_duration_from_index(index: u8) -> FrameDuration {
    match index % 6 {
        0 => FrameDuration::Ms2_5,
        1 => FrameDuration::Ms5,
        2 => FrameDuration::Ms10,
        3 => FrameDuration::Ms20,
        4 => FrameDuration::Ms40,
        _ => FrameDuration::Ms60,
    }
}

fuzz_target!(|input: FuzzInput| {
    let sample_rate = sample_rate_from_index(input.sample_rate_index);
    let channels = if input.stereo { 2 } else { 1 };
    let frame_duration = frame_duration_from_index(input.frame_duration_index);

    let config = DecoderConfig {
        sample_rate,
        channels,
        frame_duration: Some(frame_duration),
        gain: None,
    };

    let Ok(mut decoder) = shiguredo_opus::Decoder::new(config) else {
        return;
    };

    if input.data.is_empty() {
        return;
    }

    // f32 デコードがクラッシュしないことを確認する
    let _ = decoder.decode_f32(&input.data);
});
