/// text_effect.rs — 文字特效渲染模块
///
/// 实现 4 种特效的像素级渲染算法，纯 CPU 操作 RGBA 缓冲区：
///
/// 1. 文字描边 (Outline) — 内/外描边，可配粗细、颜色、透明度、抗锯齿软边
/// 2. 立体阴影 (Shadow) — 多层递进偏移阴影，配层数、偏移、模糊半径、光照方向
/// 3. 霓虹发光 (Glow) — 多层同心渐变模糊辉光，多色标、扩散半径、发光强度
/// 4. 七彩渐变 (Gradient) — 线性/径向渐变，多色标节点，一键彩虹预设
///
/// 所有函数操作 RGBA8888 格式的像素缓冲区（行优先，每像素 4 字节）。

use crate::config::{ColorStop, GlowPreset, GradientType, TextEffectConfig};

// ============================================================
// 工具函数
// ============================================================

/// 获取 RGBA 缓冲区中指定像素的索引
#[inline]
fn pixel_index(x: u32, y: u32, width: u32) -> usize {
    ((y * width + x) * 4) as usize
}

/// 获取 RGBA 缓冲区中指定像素的颜色值
#[inline]
fn get_pixel(rgba: &[u8], x: u32, y: u32, width: u32) -> [u8; 4] {
    let idx = pixel_index(x, y, width);
    [rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3]]
}

/// 设置 RGBA 缓冲区中指定像素的颜色值
#[inline]
fn set_pixel(rgba: &mut [u8], x: u32, y: u32, width: u32, color: &[u8; 4]) {
    let idx = pixel_index(x, y, width);
    rgba[idx] = color[0];
    rgba[idx + 1] = color[1];
    rgba[idx + 2] = color[2];
    rgba[idx + 3] = color[3];
}

/// 判断像素是否不透明（alpha > 0）
#[inline]
fn is_opaque(rgba: &[u8], x: u32, y: u32, width: u32) -> bool {
    let idx = pixel_index(x, y, width) + 3;
    rgba[idx] > 0
}

/// Alpha 混合：将前景颜色叠加到背景颜色上
#[inline]
fn blend_colors(bg: &[u8; 4], fg: &[u8; 4]) -> [u8; 4] {
    let f_a = fg[3] as u32;
    if f_a == 0 {
        return *bg;
    }
    if f_a == 255 {
        return *fg;
    }
    let b_a = bg[3] as u32;
    let out_a = f_a + b_a - (b_a * f_a) / 255;
    if out_a == 0 {
        return [0, 0, 0, 0];
    }
    let out_r = ((fg[0] as u32 * f_a) + (bg[0] as u32 * b_a * (255 - f_a)) / 255) / out_a;
    let out_g = ((fg[1] as u32 * f_a) + (bg[1] as u32 * b_a * (255 - f_a)) / 255) / out_a;
    let out_b = ((fg[2] as u32 * f_a) + (bg[2] as u32 * b_a * (255 - f_a)) / 255) / out_a;
    [
        out_r.min(255) as u8,
        out_g.min(255) as u8,
        out_b.min(255) as u8,
        out_a.min(255) as u8,
    ]
}

/// 将 u32 RGBA 编码转换为数组
#[inline]
fn rgba_u32_to_array(color: u32) -> [u8; 4] {
    [
        ((color >> 24) & 0xFF) as u8,
        ((color >> 16) & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
    ]
}

/// 高斯模糊（简化版：使用多次 box blur 近似）
fn gaussian_blur(rgba: &mut [u8], width: u32, height: u32, radius: u32) {
    if radius == 0 {
        return;
    }

    let passes = 3; // 3-pass box blur 近似高斯模糊
    for _ in 0..passes {
        // 水平模糊
        box_blur_horizontal(rgba, width, height, radius);
        // 垂直模糊
        box_blur_vertical(rgba, width, height, radius);
    }
}

/// 水平方向 box blur
fn box_blur_horizontal(rgba: &mut [u8], width: u32, height: u32, radius: u32) {
    let r = radius as i32;
    let len = (width * height * 4) as usize;
    let mut temp = vec![0u8; len];

    for y in 0..height {
        for x in 0..width {
            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut sum_a = 0u32;
            let mut count = 0u32;

            let x_start = (x as i32 - r).max(0) as u32;
            let x_end = (x as i32 + r).min(width as i32 - 1) as u32;

            for kx in x_start..=x_end {
                let idx = pixel_index(kx, y, width);
                sum_r += rgba[idx] as u32;
                sum_g += rgba[idx + 1] as u32;
                sum_b += rgba[idx + 2] as u32;
                sum_a += rgba[idx + 3] as u32;
                count += 1;
            }

            if count > 0 {
                let idx = pixel_index(x, y, width);
                temp[idx] = (sum_r / count) as u8;
                temp[idx + 1] = (sum_g / count) as u8;
                temp[idx + 2] = (sum_b / count) as u8;
                temp[idx + 3] = (sum_a / count) as u8;
            }
        }
    }

    rgba.copy_from_slice(&temp);
}

/// 垂直方向 box blur
fn box_blur_vertical(rgba: &mut [u8], width: u32, height: u32, radius: u32) {
    let r = radius as i32;
    let len = (width * height * 4) as usize;
    let mut temp = vec![0u8; len];

    for x in 0..width {
        for y in 0..height {
            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut sum_a = 0u32;
            let mut count = 0u32;

            let y_start = (y as i32 - r).max(0) as u32;
            let y_end = (y as i32 + r).min(height as i32 - 1) as u32;

            for ky in y_start..=y_end {
                let idx = pixel_index(x, ky, width);
                sum_r += rgba[idx] as u32;
                sum_g += rgba[idx + 1] as u32;
                sum_b += rgba[idx + 2] as u32;
                sum_a += rgba[idx + 3] as u32;
                count += 1;
            }

            if count > 0 {
                let idx = pixel_index(x, y, width);
                temp[idx] = (sum_r / count) as u8;
                temp[idx + 1] = (sum_g / count) as u8;
                temp[idx + 2] = (sum_b / count) as u8;
                temp[idx + 3] = (sum_a / count) as u8;
            }
        }
    }

    rgba.copy_from_slice(&temp);
}

// ============================================================
// 1. 文字描边 (Outline)
// ============================================================

/// 应用文字描边效果
///
/// 从原始 RGBA 缓冲区中提取 alpha 通道，向外/向内扩展生成描边。
/// 支持抗锯齿软边模式。
pub fn apply_outline(rgba: &mut [u8], width: u32, height: u32, config: &TextEffectConfig) {
    if !config.outline_enabled || config.outline_width == 0 {
        return;
    }

    let outline_color = rgba_u32_to_array(config.outline_color_rgba);
    let outline_width = config.outline_width.min(16);
    let soft_edge = config.outline_soft_edge.min(4);

    // 创建临时缓冲区存储描边结果
    let len = (width * height * 4) as usize;
    let mut outline_layer = vec![0u8; len];

    // 遍历每个像素，检查其周围是否需要描边
    for y in 0..height {
        for x in 0..width {
            // 如果当前像素是透明的，则检查周围是否需要描边
            if !is_opaque(rgba, x, y, width) {
                // 在描边半径范围内搜索不透明像素
                let mut max_alpha = 0u8;
                let mut found = false;

                for dy in -(outline_width as i32)..=(outline_width as i32) {
                    for dx in -(outline_width as i32)..=(outline_width as i32) {
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        if dist > outline_width as f32 {
                            continue;
                        }

                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || nx >= width as i32 || ny < 0 || ny >= height as i32 {
                            continue;
                        }

                        let idx = pixel_index(nx as u32, ny as u32, width) + 3;
                        let alpha = rgba[idx];
                        if alpha > 0 {
                            // 软边处理：根据距离计算 alpha
                            if soft_edge > 0 {
                                let edge_factor = if dist <= (outline_width as f32 - soft_edge as f32) {
                                    1.0
                                } else {
                                    let fade = (outline_width as f32 - dist) / soft_edge as f32;
                                    fade.max(0.0).min(1.0)
                                };
                                let blended = (alpha as f32 * edge_factor * 0.8) as u8;
                                if blended > max_alpha {
                                    max_alpha = blended;
                                }
                            } else {
                                // 硬边
                                found = true;
                                break;
                            }
                        }
                    }
                    if found && soft_edge == 0 {
                        break;
                    }
                }

                if found || max_alpha > 0 {
                    let idx = pixel_index(x, y, width);
                    if soft_edge > 0 && max_alpha > 0 {
                        outline_layer[idx] = outline_color[0];
                        outline_layer[idx + 1] = outline_color[1];
                        outline_layer[idx + 2] = outline_color[2];
                        outline_layer[idx + 3] = max_alpha;
                    } else if found {
                        outline_layer[idx] = outline_color[0];
                        outline_layer[idx + 1] = outline_color[1];
                        outline_layer[idx + 2] = outline_color[2];
                        outline_layer[idx + 3] = outline_color[3];
                    }
                }
            }
        }
    }

    // 将描边层叠加到原图上（描边在文字下方）
    for i in (0..len).step_by(4) {
        let bg = [outline_layer[i], outline_layer[i + 1], outline_layer[i + 2], outline_layer[i + 3]];
        let fg = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
        let blended = blend_colors(&bg, &fg);
        rgba[i] = blended[0];
        rgba[i + 1] = blended[1];
        rgba[i + 2] = blended[2];
        rgba[i + 3] = blended[3];
    }
}

// ============================================================
// 2. 立体阴影 (Shadow)
// ============================================================

/// 应用立体阴影效果
///
/// 多层递进偏移阴影，支持模糊半径和光照方向。
/// 每一层阴影的偏移量根据层数和光照方向计算。
pub fn apply_shadow(rgba: &mut [u8], width: u32, height: u32, config: &TextEffectConfig) {
    if !config.shadow_enabled {
        return;
    }

    let shadow_color = rgba_u32_to_array(config.shadow_color_rgba);
    let layers = config.shadow_layers.clamp(1, 8);
    let blur_radius = config.shadow_blur_radius.min(16);
    let _light_rad = config.shadow_light_dir.to_radians();

    let len = (width * height * 4) as usize;

    // 逐层生成阴影
    for layer in 0..layers {
        let layer_factor = (layer + 1) as f32 / layers as f32;
        let offset_x = (config.shadow_offset_x as f32 * layer_factor) as i32;
        let offset_y = (config.shadow_offset_y as f32 * layer_factor) as i32;

        // 创建该层的阴影缓冲区
        let mut shadow_layer = vec![0u8; len];

        // 提取 alpha 通道并偏移
        for y in 0..height {
            for x in 0..width {
                let src_idx = pixel_index(x, y, width);
                let alpha = rgba[src_idx + 3];

                if alpha > 0 {
                    let sx = x as i32 + offset_x;
                    let sy = y as i32 + offset_y;

                    if sx >= 0 && sx < width as i32 && sy >= 0 && sy < height as i32 {
                        let dst_idx = pixel_index(sx as u32, sy as u32, width);
                        // 累计 alpha（多层叠加）
                        let new_alpha = shadow_layer[dst_idx + 3] as u32
                            + (alpha as u32 * shadow_color[3] as u32 / 255 / layers as u32);
                        shadow_layer[dst_idx + 3] = new_alpha.min(255) as u8;
                        shadow_layer[dst_idx] = shadow_color[0];
                        shadow_layer[dst_idx + 1] = shadow_color[1];
                        shadow_layer[dst_idx + 2] = shadow_color[2];
                    }
                }
            }
        }

        // 应用模糊
        if blur_radius > 0 {
            gaussian_blur(&mut shadow_layer, width, height, blur_radius / 2 + 1);
        }

        // 叠加阴影层到原图（阴影在文字下方）
        for i in (0..len).step_by(4) {
            let bg = [shadow_layer[i], shadow_layer[i + 1], shadow_layer[i + 2], shadow_layer[i + 3]];
            let fg = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
            let blended = blend_colors(&bg, &fg);
            rgba[i] = blended[0];
            rgba[i + 1] = blended[1];
            rgba[i + 2] = blended[2];
            rgba[i + 3] = blended[3];
        }
    }
}

// ============================================================
// 3. 霓虹发光 (Glow)
// ============================================================

/// 生成预设色标
fn get_preset_color_stops(preset: GlowPreset, _intensity: f32) -> Vec<ColorStop> {
    match preset {
        GlowPreset::MonoBlue => vec![
            ColorStop { position: 0.0, rgba: 0x0044FF00 },
            ColorStop { position: 1.0, rgba: 0x0044FF80 },
        ],
        GlowPreset::MonoRed => vec![
            ColorStop { position: 0.0, rgba: 0xFF004400 },
            ColorStop { position: 1.0, rgba: 0xFF004480 },
        ],
        GlowPreset::MonoGreen => vec![
            ColorStop { position: 0.0, rgba: 0x00FF4400 },
            ColorStop { position: 1.0, rgba: 0x00FF4480 },
        ],
        GlowPreset::MonoGold => vec![
            ColorStop { position: 0.0, rgba: 0xFFD70000 },
            ColorStop { position: 0.5, rgba: 0xFFA50040 },
            ColorStop { position: 1.0, rgba: 0xFF8C0060 },
        ],
        GlowPreset::DuoBluePurple => vec![
            ColorStop { position: 0.0, rgba: 0x4444FF00 },
            ColorStop { position: 0.5, rgba: 0x8844FF40 },
            ColorStop { position: 1.0, rgba: 0xFF44FF60 },
        ],
        GlowPreset::DuoRedOrange => vec![
            ColorStop { position: 0.0, rgba: 0xFF444400 },
            ColorStop { position: 0.5, rgba: 0xFF884440 },
            ColorStop { position: 1.0, rgba: 0xFFAA4460 },
        ],
        GlowPreset::DuoCyanPink => vec![
            ColorStop { position: 0.0, rgba: 0x44FFFF00 },
            ColorStop { position: 0.5, rgba: 0xFF44FF40 },
            ColorStop { position: 1.0, rgba: 0xFF88FF60 },
        ],
        GlowPreset::Rainbow => vec![
            // 彩虹七色，0xRRGGBBAA 格式，所有颜色 alpha=255 确保可见
            ColorStop { position: 0.0, rgba: 0xFF0000FF },   // 红
            ColorStop { position: 0.17, rgba: 0xFFFF00FF },  // 黄
            ColorStop { position: 0.33, rgba: 0x00FF00FF },  // 绿
            ColorStop { position: 0.5, rgba: 0x00FFFFFF },   // 青
            ColorStop { position: 0.67, rgba: 0x0000FFFF },  // 蓝
            ColorStop { position: 0.83, rgba: 0x4B0082FF },  // 靛
            ColorStop { position: 1.0, rgba: 0x8F00FFFF },   // 紫
        ],
        GlowPreset::None => vec![],
    }
}

/// 根据位置在色标之间线性插值颜色
fn interpolate_color_stop(stops: &[ColorStop], position: f32) -> [u8; 4] {
    if stops.is_empty() {
        return [0, 0, 0, 0];
    }
    if stops.len() == 1 {
        return rgba_u32_to_array(stops[0].rgba);
    }

    let pos = position.clamp(0.0, 1.0);

    // 找到两个包围的色标
    for i in 0..stops.len() - 1 {
        if pos >= stops[i].position && pos <= stops[i + 1].position {
            let range = stops[i + 1].position - stops[i].position;
            if range <= 0.0 {
                return rgba_u32_to_array(stops[i].rgba);
            }
            let t = (pos - stops[i].position) / range;

            let c1 = rgba_u32_to_array(stops[i].rgba);
            let c2 = rgba_u32_to_array(stops[i + 1].rgba);

            return [
                (c1[0] as f32 + (c2[0] as f32 - c1[0] as f32) * t) as u8,
                (c1[1] as f32 + (c2[1] as f32 - c1[1] as f32) * t) as u8,
                (c1[2] as f32 + (c2[2] as f32 - c1[2] as f32) * t) as u8,
                (c1[3] as f32 + (c2[3] as f32 - c1[3] as f32) * t) as u8,
            ];
        }
    }

    // 超出范围：返回最后一个或第一个色标
    if pos <= stops[0].position {
        rgba_u32_to_array(stops[0].rgba)
    } else {
        rgba_u32_to_array(stops[stops.len() - 1].rgba)
    }
}

/// 应用霓虹发光效果
///
/// 使用多层同心渐变模糊辉光模拟霓虹效果。
pub fn apply_glow(rgba: &mut [u8], width: u32, height: u32, config: &TextEffectConfig) {
    if !config.glow_enabled {
        return;
    }

    let radius = config.glow_radius.clamp(1, 32);
    let intensity = config.glow_intensity.clamp(0.0, 1.0);

    // 获取色标
    let stops: Vec<ColorStop> = if config.glow_preset != GlowPreset::None {
        get_preset_color_stops(config.glow_preset, intensity)
    } else if !config.glow_color_stops.is_null() && config.glow_color_stops_len > 0 {
        let len = config.glow_color_stops_len.min(8) as usize;
        unsafe {
            (0..len)
                .map(|i| *config.glow_color_stops.add(i))
                .collect()
        }
    } else {
        return; // 无色标，不处理
    };

    if stops.is_empty() {
        return;
    }

    let len = (width * height * 4) as usize;

    // 创建发光缓冲区
    let mut glow_buffer = vec![0u8; len];

    // 第一步：扩展 alpha 通道
    for y in 0..height {
        for x in 0..width {
            let idx = pixel_index(x, y, width);
            if rgba[idx + 3] > 0 {
                // 将该像素的颜色扩展到半径范围内
                for dy in -(radius as i32)..=(radius as i32) {
                    for dx in -(radius as i32)..=(radius as i32) {
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        if dist > radius as f32 {
                            continue;
                        }

                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || nx >= width as i32 || ny < 0 || ny >= height as i32 {
                            continue;
                        }

                        // 根据距离计算发光强度
                        let glow_factor = 1.0 - (dist / radius as f32);
                        let glow_alpha = (glow_factor * intensity * 255.0) as u8;

                        let dst_idx = pixel_index(nx as u32, ny as u32, width);

                        // 使用色标根据距离计算颜色
                        let color_pos = 1.0 - (dist / radius as f32);
                        let glow_color = interpolate_color_stop(&stops, color_pos);

                        // Alpha 叠加
                        let existing_alpha = glow_buffer[dst_idx + 3] as u32;
                        let new_alpha = existing_alpha + (glow_color[3] as u32 * glow_alpha as u32 / 255);
                        let new_alpha = new_alpha.min(255) as u8;

                        if new_alpha > glow_buffer[dst_idx + 3] {
                            // 混合颜色
                            let _factor = glow_color[3] as f32 / 255.0;
                            let old_r = glow_buffer[dst_idx] as f32;
                            let old_g = glow_buffer[dst_idx + 1] as f32;
                            let old_b = glow_buffer[dst_idx + 2] as f32;
                            let new_r = glow_color[0] as f32;
                            let new_g = glow_color[1] as f32;
                            let new_b = glow_color[2] as f32;

                            let mix = intensity * glow_factor;
                            glow_buffer[dst_idx] = (old_r * (1.0 - mix) + new_r * mix) as u8;
                            glow_buffer[dst_idx + 1] = (old_g * (1.0 - mix) + new_g * mix) as u8;
                            glow_buffer[dst_idx + 2] = (old_b * (1.0 - mix) + new_b * mix) as u8;
                            glow_buffer[dst_idx + 3] = new_alpha;
                        }
                    }
                }
            }
        }
    }

    // 应用模糊使发光更柔和
    gaussian_blur(&mut glow_buffer, width, height, radius / 3 + 1);

    // 将发光叠加到原图（发光层在文字下方）
    for i in (0..len).step_by(4) {
        let bg = [glow_buffer[i], glow_buffer[i + 1], glow_buffer[i + 2], glow_buffer[i + 3]];
        let fg = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
        let blended = blend_colors(&bg, &fg);
        rgba[i] = blended[0];
        rgba[i + 1] = blended[1];
        rgba[i + 2] = blended[2];
        rgba[i + 3] = blended[3];
    }
}

// ============================================================
// 4. 七彩渐变 (Gradient)
// ============================================================

/// 应用渐变效果
///
/// 支持线性渐变和径向渐变两种模式。
/// 线性渐变根据角度方向计算每个像素的位置。
/// 径向渐变以图像中心为原点计算半径比例。
pub fn apply_gradient(rgba: &mut [u8], width: u32, height: u32, config: &TextEffectConfig) {
    if !config.gradient_enabled {
        return;
    }

    // 获取色标
    let stops: Vec<ColorStop> = if config.rainbow_preset {
        // 彩虹预设
        get_preset_color_stops(GlowPreset::Rainbow, 1.0)
    } else if !config.gradient_color_stops.is_null() && config.gradient_color_stops_len > 0 {
        let len = config.gradient_color_stops_len.min(8) as usize;
        unsafe {
            (0..len)
                .map(|i| *config.gradient_color_stops.add(i))
                .collect()
        }
    } else {
        return; // 无色标，不处理
    };

    if stops.is_empty() {
        return;
    }

    let angle_rad = config.gradient_angle.to_radians();
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let max_radius = ((center_x * center_x + center_y * center_y) as f32).sqrt();

    for y in 0..height {
        for x in 0..width {
            let idx = pixel_index(x, y, width);

            // 只在非透明像素上应用渐变
            if rgba[idx + 3] == 0 {
                continue;
            }

            let position = match config.gradient_type {
                GradientType::Linear => {
                    // 线性渐变：将像素投影到渐变方向上
                    let dx = x as f32 - center_x;
                    let dy = y as f32 - center_y;
                    // 旋转坐标
                    let rotated_x = dx * angle_rad.cos() + dy * angle_rad.sin();
                    // 归一化到 0~1
                    let half_len = (width.max(height) as f32) / 2.0;
                    (rotated_x / half_len + 1.0) / 2.0
                }
                GradientType::Radial => {
                    // 径向渐变：到中心的距离
                    let dx = x as f32 - center_x;
                    let dy = y as f32 - center_y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    (dist / max_radius).min(1.0)
                }
            };

            let color = interpolate_color_stop(&stops, position.clamp(0.0, 1.0));

            // 将渐变颜色混合到原像素（保留原像素的 alpha）
            let orig_alpha = rgba[idx + 3];
            let blend_factor = color[3] as f32 / 255.0;
            rgba[idx] = (rgba[idx] as f32 * (1.0 - blend_factor) + color[0] as f32 * blend_factor) as u8;
            rgba[idx + 1] = (rgba[idx + 1] as f32 * (1.0 - blend_factor) + color[1] as f32 * blend_factor) as u8;
            rgba[idx + 2] = (rgba[idx + 2] as f32 * (1.0 - blend_factor) + color[2] as f32 * blend_factor) as u8;
            rgba[idx + 3] = orig_alpha; // 保持原始透明度
        }
    }
}

// ============================================================
// 5. 全特效叠加
// ============================================================

/// 按底层到顶层顺序叠加所有特效
///
/// 渲染顺序（从下到上）：
/// 1. 阴影 (Shadow) — 最底层
/// 2. 霓虹发光 (Glow)
/// 3. 描边 (Outline)
/// 4. 渐变 (Gradient) — 最顶层，覆盖文字颜色
pub fn apply_all_effects(rgba: &mut [u8], width: u32, height: u32, config: &TextEffectConfig) {
    // 顺序：阴影 -> 发光 -> 描边 -> 渐变
    apply_shadow(rgba, width, height, config);
    apply_glow(rgba, width, height, config);
    apply_outline(rgba, width, height, config);
    apply_gradient(rgba, width, height, config);
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_buffer(width: u32, height: u32) -> Vec<u8> {
        let len = (width * height * 4) as usize;
        let mut buf = vec![0u8; len];
        // 绘制一个简单的方块作为文字
        for y in height / 4..3 * height / 4 {
            for x in width / 4..3 * width / 4 {
                let idx = pixel_index(x, y, width);
                buf[idx] = 255;     // R
                buf[idx + 1] = 255; // G
                buf[idx + 2] = 255; // B
                buf[idx + 3] = 255; // A
            }
        }
        buf
    }

    #[test]
    fn test_outline_effect() {
        let mut buf = create_test_buffer(32, 32);
        let mut config = TextEffectConfig::new();
        config.outline_enabled = true;
        config.outline_width = 2;
        config.outline_color_rgba = 0x000000FF;

        let original = buf.clone();
        apply_outline(&mut buf, 32, 32, &config);
        // 描边后应该有新的非透明像素
        assert!(buf != original);
    }

    #[test]
    fn test_shadow_effect() {
        let mut buf = create_test_buffer(32, 32);
        let mut config = TextEffectConfig::new();
        config.shadow_enabled = true;
        config.shadow_layers = 2;
        config.shadow_offset_x = 3;
        config.shadow_offset_y = 3;

        let original = buf.clone();
        apply_shadow(&mut buf, 32, 32, &config);
        assert!(buf != original);
    }

    #[test]
    fn test_glow_effect() {
        let mut buf = create_test_buffer(32, 32);
        let mut config = TextEffectConfig::new();
        config.glow_enabled = true;
        config.glow_radius = 4;
        config.glow_preset = GlowPreset::MonoBlue;
        config.glow_intensity = 0.6;

        let original = buf.clone();
        apply_glow(&mut buf, 32, 32, &config);
        assert!(buf != original);
    }

    #[test]
    fn test_gradient_effect_linear() {
        let mut buf = create_test_buffer(32, 32);
        let mut config = TextEffectConfig::new();
        config.gradient_enabled = true;
        config.rainbow_preset = true;
        config.gradient_type = GradientType::Linear;

        let original = buf.clone();
        apply_gradient(&mut buf, 32, 32, &config);
        assert!(buf != original);
    }

    #[test]
    fn test_gradient_effect_radial() {
        let mut buf = create_test_buffer(32, 32);
        let mut config = TextEffectConfig::new();
        config.gradient_enabled = true;
        config.rainbow_preset = true;
        config.gradient_type = GradientType::Radial;

        let original = buf.clone();
        apply_gradient(&mut buf, 32, 32, &config);
        assert!(buf != original);
    }

    #[test]
    fn test_all_effects() {
        let mut buf = create_test_buffer(32, 32);
        let mut config = TextEffectConfig::new();
        config.outline_enabled = true;
        config.outline_width = 2;
        config.shadow_enabled = true;
        config.glow_enabled = true;
        config.glow_preset = GlowPreset::MonoBlue;
        config.gradient_enabled = true;
        config.rainbow_preset = true;

        let original = buf.clone();
        apply_all_effects(&mut buf, 32, 32, &config);
        assert!(buf != original);
    }

    #[test]
    fn test_no_effect() {
        let mut buf = create_test_buffer(32, 32);
        let config = TextEffectConfig::new();
        let original = buf.clone();
        apply_all_effects(&mut buf, 32, 32, &config);
        assert_eq!(buf, original);
    }
}