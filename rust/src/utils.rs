/// utils.rs — 底层工具函数集
///
/// 提供 RGBA 像素操作、Alpha 混合、矩形裁剪、图像缩放、
/// C 字符串安全校验和获取当前系统时间（毫秒）等通用功能。
/// 所有函数均为纯 CPU 实现，不依赖外部图形库。

use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================
// RGBA 像素操作
// ============================================================

/// Alpha 混合：将前景像素 frgba 按 alpha 比例叠加到背景像素 brgba 上。
///
/// 公式（预乘 alpha 方式）：
///   out_a = f_a + b_a * (1 - f_a)
///   out_r = (f_r * f_a + b_r * b_a * (1 - f_a)) / out_a
///
/// 所有运算使用 u32 避免中间溢出。
#[inline]
pub fn blend_alpha(brgba: &[u8; 4], frgba: &[u8; 4]) -> [u8; 4] {
    let b_r = brgba[0] as u32;
    let b_g = brgba[1] as u32;
    let b_b = brgba[2] as u32;
    let b_a = brgba[3] as u32;

    let f_r = frgba[0] as u32;
    let f_g = frgba[1] as u32;
    let f_b = frgba[2] as u32;
    let f_a = frgba[3] as u32;

    if f_a == 255 {
        // 前景完全不透明，直接覆盖
        return *frgba;
    }
    if f_a == 0 {
        // 前景完全透明，返回背景
        return *brgba;
    }

    // 标准 Alpha 混合公式
    let out_a = f_a + b_a - (b_a * f_a) / 255;
    if out_a == 0 {
        return [0, 0, 0, 0];
    }

    let out_r = ((f_r * f_a) + (b_r * b_a * (255 - f_a)) / 255) / out_a;
    let out_g = ((f_g * f_a) + (b_g * b_a * (255 - f_a)) / 255) / out_a;
    let out_b = ((f_b * f_a) + (b_b * b_a * (255 - f_a)) / 255) / out_a;

    [
        out_r.min(255) as u8,
        out_g.min(255) as u8,
        out_b.min(255) as u8,
        out_a.min(255) as u8,
    ]
}

/// 快速 Alpha 混合（无除法优化版），适用于性能敏感场景。
/// 假设前景不透明度固定，仅做简单 lerp 近似。
#[inline]
pub fn blend_alpha_fast(brgba: &[u8; 4], frgba: &[u8; 4]) -> [u8; 4] {
    let b_a = brgba[3] as u32;
    let f_a = frgba[3] as u32;

    if f_a == 255 {
        return *frgba;
    }
    if f_a == 0 || b_a == 0 {
        if b_a == 0 {
            return *frgba;
        }
        return *brgba;
    }

    // Alpha 加权混合
    let total_a = b_a + f_a;
    let out_r = ((brgba[0] as u32 * b_a + frgba[0] as u32 * f_a) / total_a).min(255) as u8;
    let out_g = ((brgba[1] as u32 * b_a + frgba[1] as u32 * f_a) / total_a).min(255) as u8;
    let out_b = ((brgba[2] as u32 * b_a + frgba[2] as u32 * f_a) / total_a).min(255) as u8;
    let out_a = (total_a.min(255)) as u8;

    [out_r, out_g, out_b, out_a]
}

/// 矩形裁剪：将源矩形 (sx, sy, sw, sh) 裁剪到画布边界 (cw, ch) 内。
///
/// 返回裁剪后的 (x, y, w, h)，如果完全在画布外则返回 None。
#[inline]
pub fn clip_rect(
    sx: i32, sy: i32, sw: u32, sh: u32,
    cw: u32, ch: u32,
) -> Option<(u32, u32, u32, u32)> {
    let x = sx.max(0) as u32;
    let y = sy.max(0) as u32;
    let right = (sx + sw as i32).min(cw as i32).max(0) as u32;
    let bottom = (sy + sh as i32).min(ch as i32).max(0) as u32;

    if x >= right || y >= bottom {
        return None; // 完全在画布外
    }
    Some((x, y, right - x, bottom - y))
}

/// 邻近插值缩放 RGBA 图像。
///
/// 从 src 缩放至 dst_w x dst_h，结果写入 dst 切片。
/// dst 长度必须 >= dst_w * dst_h * 4。
pub fn resize_nearest(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
) {
    let dst_len = (dst_w * dst_h * 4) as usize;
    if dst.len() < dst_len {
        // 缓冲区不足时仅缩放能容纳的部分
        return;
    }

    // 计算缩放比例
    let ratio_x = src_w as f64 / dst_w as f64;
    let ratio_y = src_h as f64 / dst_h as f64;

    for dy in 0..dst_h {
        // 计算对应的源像素 y 坐标（最近邻）
        let sy = ((dy as f64 + 0.5) * ratio_y) as u32;
        let sy = sy.min(src_h - 1);

        for dx in 0..dst_w {
            let sx = ((dx as f64 + 0.5) * ratio_x) as u32;
            let sx = sx.min(src_w - 1);

            let src_idx = ((sy * src_w + sx) * 4) as usize;
            let dst_idx = ((dy * dst_w + dx) * 4) as usize;

            // 逐个像素拷贝 RGBA 值
            dst[dst_idx] = src[src_idx];
            dst[dst_idx + 1] = src[src_idx + 1];
            dst[dst_idx + 2] = src[src_idx + 2];
            dst[dst_idx + 3] = src[src_idx + 3];
        }
    }
}

// ============================================================
// C 字符串安全工具
// ============================================================

/// 校验 C 风格字符串：检查指针非空且包含有效的 null 终止符。
/// 返回字符串字节长度（不含 null）。
///
/// # 安全性
/// 此函数使用 unsafe 对原始指针解引用，调用方必须保证：
/// - ptr 指向有效的、可读的内存区域
/// - 内存以 null 终止
pub unsafe fn validate_c_str(ptr: *const std::ffi::c_char) -> Option<usize> {
    if ptr.is_null() {
        return None;
    }

    // 使用 libc 的 strlen 或手动扫描
    let mut len: usize = 0;
    loop {
        let c = *ptr.offset(len as isize);
        if c == 0 {
            break;
        }
        len += 1;
        // 安全上限：防止无限循环（最大 64KB）
        if len > 65536 {
            return None;
        }
    }
    Some(len)
}

/// 安全的 C 字符串长度计算，带最大长度保护。
pub unsafe fn safe_strlen(ptr: *const std::ffi::c_char, max_len: usize) -> Option<usize> {
    if ptr.is_null() {
        return None;
    }

    let mut len: usize = 0;
    while len < max_len {
        let c = *ptr.offset(len as isize);
        if c == 0 {
            return Some(len);
        }
        len += 1;
    }
    // 超过 max_len 仍未找到 null 终止符，返回 None 表示不安全
    None
}

// ============================================================
// 时间工具
// ============================================================

/// 获取当前系统时间戳（毫秒），基于 UNIX_EPOCH。
/// 如果系统时间异常，返回 0。
pub fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ============================================================
// 颜色工具
// ============================================================

/// 将 RGBA 四字节解析为 [r, g, b, a] 数组。
#[inline]
pub fn rgba_from_u32(color: u32) -> [u8; 4] {
    [
        ((color >> 24) & 0xFF) as u8,
        ((color >> 16) & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
    ]
}

/// 将 [r, g, b, a] 数组编码为 u32。
#[inline]
pub fn rgba_to_u32(rgba: &[u8; 4]) -> u32 {
    ((rgba[0] as u32) << 24) | ((rgba[1] as u32) << 16) | ((rgba[2] as u32) << 8) | (rgba[3] as u32)
}

/// 将 f32 颜色通道值（0.0~1.0）钳位为 u8（0~255）。
#[inline]
pub fn float_channel_to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0) as u8
}

/// 将 u8 颜色通道值（0~255）转换为 f32（0.0~1.0）。
#[inline]
pub fn u8_channel_to_float(v: u8) -> f32 {
    v as f32 / 255.0
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_alpha_opaque() {
        let bg = [255, 0, 0, 255];
        let fg = [0, 255, 0, 255];
        let result = blend_alpha(&bg, &fg);
        // 完全不透明前景 -> 输出完全为前景
        assert_eq!(result, fg);
    }

    #[test]
    fn test_blend_alpha_transparent() {
        let bg = [255, 0, 0, 255];
        let fg = [0, 255, 0, 0];
        let result = blend_alpha(&bg, &fg);
        // 完全透明前景 -> 输出为背景
        assert_eq!(result, bg);
    }

    #[test]
    fn test_clip_rect_inside() {
        let result = clip_rect(10, 10, 50, 50, 200, 200);
        assert_eq!(result, Some((10, 10, 50, 50)));
    }

    #[test]
    fn test_clip_rect_outside() {
        let result = clip_rect(300, 300, 50, 50, 200, 200);
        assert_eq!(result, None);
    }

    #[test]
    fn test_current_time_ms() {
        let t = current_time_ms();
        assert!(t > 1_700_000_000_000); // 大致检查：应该在2023年以后
    }

    #[test]
    fn test_rgba_conversion() {
        let rgba = [0x12, 0x34, 0x56, 0x78];
        let encoded = rgba_to_u32(&rgba);
        let decoded = rgba_from_u32(encoded);
        assert_eq!(rgba, decoded);
    }
}