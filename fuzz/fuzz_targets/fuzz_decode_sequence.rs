#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use shiguredo_opus::{DecoderConfig, FrameDuration};

/// デコードシーケンスで使用する操作の種類
#[derive(Arbitrary, Debug)]
enum DecodeOp {
    /// 通常のデコード
    Decode { data: Vec<u8> },
    /// FEC デコード
    DecodeFec { data: Vec<u8> },
    /// PLC (パケットロス補間)
    DecodePlc,
    /// f32 デコード
    DecodeF32 { data: Vec<u8> },
    /// FEC f32 デコード
    DecodeFecF32 { data: Vec<u8> },
    /// PLC f32
    DecodePlcF32,
    /// i24 デコード
    DecodeI24 { data: Vec<u8> },
    /// FEC i24 デコード
    DecodeFecI24 { data: Vec<u8> },
    /// PLC i24
    DecodePlcI24,
    /// デコーダーリセット
    Reset,
}

/// ファジング入力
#[derive(Arbitrary, Debug)]
struct FuzzInput {
    /// サンプルレート選択
    sample_rate_index: u8,
    /// チャンネル数
    stereo: bool,
    /// フレーム時間選択
    frame_duration_index: u8,
    /// デコード操作のシーケンス
    ops: Vec<DecodeOp>,
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

    // 操作数を制限してタイムアウトを防ぐ
    if input.ops.len() > 32 {
        return;
    }

    let config = DecoderConfig {
        sample_rate,
        channels,
        frame_duration: Some(frame_duration),
        gain: None,
    };

    let Ok(mut decoder) = shiguredo_opus::Decoder::new(config) else {
        return;
    };

    // 操作シーケンスを順に実行する
    for op in &input.ops {
        match op {
            DecodeOp::Decode { data } => {
                if !data.is_empty() {
                    let _ = decoder.decode(data);
                }
            }
            DecodeOp::DecodeFec { data } => {
                if !data.is_empty() {
                    let _ = decoder.decode_fec(data);
                }
            }
            DecodeOp::DecodePlc => {
                let _ = decoder.decode_plc();
            }
            DecodeOp::DecodeF32 { data } => {
                if !data.is_empty() {
                    let _ = decoder.decode_f32(data);
                }
            }
            DecodeOp::DecodeFecF32 { data } => {
                if !data.is_empty() {
                    let _ = decoder.decode_fec_f32(data);
                }
            }
            DecodeOp::DecodePlcF32 => {
                let _ = decoder.decode_plc_f32();
            }
            DecodeOp::DecodeI24 { data } => {
                if !data.is_empty() {
                    let _ = decoder.decode_i24(data);
                }
            }
            DecodeOp::DecodeFecI24 { data } => {
                if !data.is_empty() {
                    let _ = decoder.decode_fec_i24(data);
                }
            }
            DecodeOp::DecodePlcI24 => {
                let _ = decoder.decode_plc_i24();
            }
            DecodeOp::Reset => {
                let _ = decoder.reset();
            }
        }
    }
});
