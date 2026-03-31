#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use shiguredo_opus::{Application, DecoderConfig, EncoderConfig, FrameDuration};

/// ファジング入力からエンコード/デコード設定を決定するための構造体
#[derive(Arbitrary, Debug)]
struct FuzzInput {
    /// サンプルレート選択
    sample_rate_index: u8,
    /// チャンネル数
    stereo: bool,
    /// フレーム時間選択
    frame_duration_index: u8,
    /// アプリケーションモード選択
    application_index: u8,
    /// エンコード形式 (0: i16, 1: f32, 2: i24)
    encode_format: u8,
    /// ランダム PCM データ (i16 として解釈)
    pcm_data: Vec<u8>,
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

/// インデックスから Application を取得する
fn application_from_index(index: u8) -> Application {
    match index % 3 {
        0 => Application::Voip,
        1 => Application::Audio,
        _ => Application::LowDelay,
    }
}

fuzz_target!(|input: FuzzInput| {
    let sample_rate = sample_rate_from_index(input.sample_rate_index);
    let channels: u8 = if input.stereo { 2 } else { 1 };
    let frame_duration = frame_duration_from_index(input.frame_duration_index);
    let application = application_from_index(input.application_index);
    let frame_samples = match frame_duration {
        FrameDuration::Ms2_5 => sample_rate as usize / 400,
        FrameDuration::Ms5 => sample_rate as usize / 200,
        FrameDuration::Ms10 => sample_rate as usize / 100,
        FrameDuration::Ms20 => sample_rate as usize / 50,
        FrameDuration::Ms40 => sample_rate as usize / 25,
        FrameDuration::Ms60 => sample_rate as usize * 3 / 50,
    };
    let total_samples = frame_samples * channels as usize;

    if total_samples == 0 {
        return;
    }

    let encoder_config = EncoderConfig {
        sample_rate,
        channels,
        bitrate: None,
        application: Some(application),
        frame_duration: Some(frame_duration),
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
    };

    let Ok(mut encoder) = shiguredo_opus::Encoder::new(encoder_config) else {
        return;
    };

    let decoder_config = DecoderConfig {
        sample_rate,
        channels,
        frame_duration: Some(frame_duration),
        gain: None,
    };

    let Ok(mut decoder) = shiguredo_opus::Decoder::new(decoder_config) else {
        return;
    };

    // ランダムバイト列から PCM データを生成してエンコード→デコードする
    match input.encode_format % 3 {
        0 => {
            // i16 エンコード
            if input.pcm_data.len() < total_samples * 2 {
                return;
            }
            let pcm: Vec<i16> = input.pcm_data[..total_samples * 2]
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect();
            let Ok(encoded) = encoder.encode(&pcm) else {
                return;
            };
            let _ = decoder.decode(&encoded);
        }
        1 => {
            // f32 エンコード
            if input.pcm_data.len() < total_samples * 4 {
                return;
            }
            let pcm: Vec<f32> = input.pcm_data[..total_samples * 4]
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let Ok(encoded) = encoder.encode_f32(&pcm) else {
                return;
            };
            let _ = decoder.decode_f32(&encoded);
        }
        _ => {
            // i24 エンコード
            if input.pcm_data.len() < total_samples * 4 {
                return;
            }
            let pcm: Vec<i32> = input.pcm_data[..total_samples * 4]
                .chunks_exact(4)
                .map(|c| {
                    // 下位 24bit のみ使用する
                    let v = i32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                    v & 0x00FF_FFFF
                })
                .collect();
            let Ok(encoded) = encoder.encode_i24(&pcm) else {
                return;
            };
            let _ = decoder.decode_i24(&encoded);
        }
    }
});
