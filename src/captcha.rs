use std::collections::HashMap;
use std::io::Write;
use std::sync::OnceLock;

use image::imageops::FilterType;
use image::load_from_memory;
use ort::session::{RunOptions, Session};
use ort::value::Value;

/// 模型文件（轻量模型，由 ddddocr 的 common_old.onnx 重命名而来）。
const MODEL_PATH: &str = "models/common_lite.onnx";

/// 学校验证码固定 4 位字符。
const MAX_CAPTCHA_LEN: usize = 4;

/// 字符集（与 common_lite.onnx 的 8210 类输出对齐，即 ddddocr 的 CHARSET_OLD）。
static CHARSET: [&str; 8210] = include!("../charset.json");

/// 手动模式：把验证码 PNG 保存到文件。
pub fn save_png(bytes: &[u8], path: &str) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)
}

/// 全角字母/数字 → 半角 ASCII。
fn fullwidth_to_ascii(c: char) -> char {
    match c {
        '０'..='９' => char::from_u32(c as u32 - 0xFF10 + 0x30).unwrap_or(c),
        'Ａ'..='Ｚ' => char::from_u32(c as u32 - 0xFF21 + 0x41).unwrap_or(c),
        'ａ'..='ｚ' => char::from_u32(c as u32 - 0xFF41 + 0x61).unwrap_or(c),
        _ => c,
    }
}

/// 是否是有效验证码字符（字母或数字，含全角形式）。
fn is_valid_captcha_char(c: char) -> bool {
    fullwidth_to_ascii(c).is_ascii_alphanumeric()
}

/// 有效类索引掩码（字母数字 + blank），由 charset.json 预计算一次并缓存。
fn valid_mask() -> &'static Vec<bool> {
    static MASK: OnceLock<Vec<bool>> = OnceLock::new();
    MASK.get_or_init(|| {
        CHARSET
            .iter()
            .enumerate()
            .map(|(i, s)| i == 0 || s.chars().next().map(is_valid_captcha_char).unwrap_or(false))
            .collect()
    })
}

/// 归一化：全角→半角、统一转大写、仅保留 ASCII 字母数字。
fn normalize(raw: &str) -> String {
    raw.chars()
        .map(fullwidth_to_ascii)
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// 归一化 + 固定长度截断（学校验证码固定 4 位）。
fn postprocess(raw: &str) -> String {
    normalize(raw).chars().take(MAX_CAPTCHA_LEN).collect()
}

/// 缓存的 ONNX 会话：进程内只加载一次模型，避免每次识别都重读磁盘。
fn session() -> Option<&'static tokio::sync::Mutex<Session>> {
    static SESSION: OnceLock<Option<tokio::sync::Mutex<Session>>> = OnceLock::new();
    SESSION
        .get_or_init(|| {
            Session::builder()
                .ok()
                .and_then(|mut b| b.commit_from_file(MODEL_PATH).ok())
                .map(tokio::sync::Mutex::new)
        })
        .as_ref()
}

/// 自动识别验证码（ort + common_lite.onnx，复刻 ddddocr 的 classification）。
/// 失败时返回 None，由调用方回退手动模式。
pub async fn recognize(png: &[u8]) -> Option<String> {
    let png = png.to_vec();
    let (h, w, data) = tokio::task::spawn_blocking(move || preprocess(&png))
        .await
        .ok()??;

    let input_value = Value::from_array((vec![1usize, 1, h, w], data)).ok()?;
    let inputs = HashMap::from([("input1".to_string(), input_value)]);
    let run_options = RunOptions::new().ok()?;
    let mut session = session()?.lock().await;
    let outputs = session.run_async(inputs, &run_options).ok()?.await.ok()?;

    // 输出 shape: (seq_len, 1, 8210)，flatten 后每 timestep 对 8210 类做 argmax
    let output = &outputs[0];
    let (_, output_data) = output.try_extract_tensor::<f32>().ok()?;
    let num_classes = CHARSET.len();
    // 有效索引：仅字母数字（含全角），用于限制 argmax，避免误识别为汉字
    let valid = valid_mask();
    let seq_len = output_data.len() / num_classes;
    let mut raw = String::new();
    let mut prev = usize::MAX;
    for t in 0..seq_len {
        let row = &output_data[t * num_classes..(t + 1) * num_classes];
        let mut max_idx = usize::MAX;
        let mut max_val = f32::NEG_INFINITY;
        for (i, &v) in row.iter().enumerate() {
            if valid[i] && v > max_val {
                max_val = v;
                max_idx = i;
            }
        }
        if max_idx == usize::MAX || max_idx == prev {
            continue; // 无有效类 或 连续重复
        }
        prev = max_idx;
        if max_idx == 0 {
            continue; // blank
        }
        raw.push_str(CHARSET[max_idx]);
    }
    Some(postprocess(&raw))
}

fn preprocess(png: &[u8]) -> Option<(usize, usize, Vec<f32>)> {
    let img = load_from_memory(png).ok()?;
    if img.width() == 0 || img.height() == 0 {
        return None;
    }
    let new_width = (img.width() as f32 * (64.0 / img.height() as f32))
        .round()
        .max(1.0) as u32;
    let resized = img.resize_exact(new_width, 64, FilterType::Lanczos3);
    let gray = resized.to_luma8();
    let h = gray.height() as usize;
    let w = gray.width() as usize;
    let data = gray
        .pixels()
        .map(|pixel| (pixel[0] as f32 / 255.0 - 0.5) / 0.5)
        .collect();
    Some((h, w, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recognize_runs() {
        // 生成一张带噪声的图片，验证模型能加载并完成推理（结果内容不保证准确）
        let mut img = image::RgbImage::new(120, 50);
        for p in img.pixels_mut() {
            *p = image::Rgb([255, 255, 255]);
        }
        for y in 0..50u32 {
            for x in 0..120u32 {
                if (x + y) % 11 == 0 {
                    img.put_pixel(x, y, image::Rgb([0, 0, 0]));
                }
            }
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        let result = recognize(&buf.into_inner()).await;
        assert!(result.is_some(), "识别应返回结果");
    }

    #[test]
    fn normalize_works() {
        assert_eq!(normalize("aB3"), "AB3");
        assert_eq!(normalize("Ａｂ３x"), "AB3X");
        assert_eq!(normalize("a中3b"), "A3B");
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn postprocess_works() {
        assert_eq!(postprocess("aB3xZ47"), "AB3X"); // 超过 4 位截断
        assert_eq!(postprocess("Ａｂ３"), "AB3"); // 不足 4 位保持
    }
}
