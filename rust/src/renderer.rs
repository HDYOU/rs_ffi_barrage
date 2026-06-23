/// renderer.rs — CPU RGBA 渲染管线
///
/// 渲染器是弹幕引擎的核心渲染模块，负责：
/// - 透明画布的创建和管理
/// - 文本渲染（模拟位图字体或简单字符轮廓——实际项目中应接入字体引擎）
/// - Emoji 图文混排（垂直居中）
/// - 视口裁剪
/// - Alpha 透明混合
/// - 输出标准 RGBA8888 像素缓冲区
/// - SIMD 加速坐标与像素混合
/// - 文字+Emoji 混合排版布局

use crate::barrage::BarrageObject;
use crate::config::TextEffectConfig;
use crate::emoji::EmojiManager;
use crate::utils;

use std::sync::Arc;

// ============================================================
// 渲染选项
// ============================================================

/// 渲染选项
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    /// 是否启用特效
    pub enable_effects: bool,
    /// 是否使用全局特效配置
    pub use_global_effects: bool,
    /// 全局透明度
    pub global_alpha: f32,
    /// 画布背景色（RGBA，默认全透明）
    pub background_color: u32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            enable_effects: true,
            use_global_effects: true,
            global_alpha: 1.0,
            background_color: 0x00000000,
        }
    }
}

// ============================================================
// 简单位图字体模拟（用于无外部字体时的回退渲染）
// ============================================================

/// 简单的位图字体渲染（8x8 像素 ASCII 字符）
///
/// 实际项目中应使用 freetype/rusttype 等字体引擎。
/// 此处提供占位实现，渲染弹幕文本的矩形区域。
struct BitmapFont;

impl BitmapFont {
    /// 测量文本的宽度和高度
    fn measure_text(text: &str, font_size: u32) -> (u32, u32) {
        let char_width = font_size / 2;
        let text_len = text.chars().count() as u32;
        (text_len * char_width, font_size)
    }

    /// 渲染文本到 RGBA 缓冲区（仅绘制矩形占位）
    fn render_text(
        rgba: &mut [u8],
        text: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        color: &[u8; 4],
        font_size: u32,
    ) {
        let char_width = font_size / 2;
        let char_height = font_size;

        for (i, _ch) in text.chars().enumerate() {
            let cx = x + (i as u32 * char_width) as i32;
            let cy = y;

            // 绘制字符的矩形区域
            for dy in 0..char_height {
                for dx in 0..char_width {
                    let px = cx + dx as i32;
                    let py = cy + dy as i32;

                    if px < 0 || px >= width as i32 || py < 0 || py >= height as i32 {
                        continue;
                    }

                    let idx = ((py as u32 * width + px as u32) * 4) as usize;
                    if idx + 3 >= rgba.len() {
                        continue;
                    }

                    // 简单的点阵效果（边缘留空模拟字符形状）
                    let is_edge = dx == 0 || dx == char_width - 1 || dy == 0 || dy == char_height - 1;
                    if !is_edge {
                        // 混合颜色
                        let bg = [rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3]];
                        let blended = utils::blend_alpha_fast(&bg, color);
                        rgba[idx] = blended[0];
                        rgba[idx + 1] = blended[1];
                        rgba[idx + 2] = blended[2];
                        rgba[idx + 3] = blended[3];
                    }
                }
            }
        }
    }
}

// ============================================================
// 渲染器
// ============================================================

/// CPU RGBA 渲染管线
///
/// 核心渲染引擎，管理画布并执行所有渲染操作。
pub struct Renderer {
    /// 画布宽度
    canvas_width: u32,
    /// 画布高度
    canvas_height: u32,
    /// 渲染缓冲区
    frame_buffer: Vec<u8>,
    /// 渲染选项
    options: RenderOptions,
    /// 全局特效配置（当 use_global_effects 为 true 时使用）
    global_effect_config: Option<Arc<TextEffectConfig>>,
    /// 渲染统计信息
    pub stats: RenderStats,
}

/// 渲染统计
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStats {
    /// 已渲染的弹幕数量
    pub barrages_rendered: u32,
    /// 已渲染的 Emoji 数量
    pub emojis_rendered: u32,
    /// 渲染耗时（微秒）
    pub render_time_us: u64,
}

impl Renderer {
    /// 创建新的渲染器
    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        let buffer_size = (canvas_width * canvas_height * 4) as usize;
        Self {
            canvas_width,
            canvas_height,
            frame_buffer: vec![0u8; buffer_size],
            options: RenderOptions::default(),
            global_effect_config: None,
            stats: RenderStats::default(),
        }
    }

    /// 创建带背景色的渲染器
    pub fn with_background(canvas_width: u32, canvas_height: u32, bg_color: u32) -> Self {
        let mut renderer = Self::new(canvas_width, canvas_height);
        renderer.options.background_color = bg_color;
        renderer.clear_buffer();
        renderer
    }

    // ===== 画布管理 =====

    /// 清空画布（使用背景色填充）
    pub fn clear_buffer(&mut self) {
        let bg = utils::rgba_from_u32(self.options.background_color);
        let len = self.frame_buffer.len();
        // 使用 SIMD 友好的方式填充
        for chunk in self.frame_buffer.chunks_exact_mut(4) {
            chunk.copy_from_slice(&bg);
        }
        // 处理剩余字节
        if len % 4 != 0 {
            let remainder = &mut self.frame_buffer[len - (len % 4)..];
            for (i, v) in remainder.iter_mut().enumerate() {
                *v = bg[i % 4];
            }
        }
    }

    /// 获取帧缓冲区引用
    pub fn frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    /// 获取帧缓冲区可变引用
    pub fn frame_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.frame_buffer
    }

    /// 调整画布大小
    pub fn resize(&mut self, width: u32, height: u32) {
        let new_size = (width * height * 4) as usize;
        self.canvas_width = width;
        self.canvas_height = height;
        self.frame_buffer.resize(new_size, 0);
        self.clear_buffer();
    }

    // ===== 选项配置 =====

    /// 设置渲染选项
    pub fn set_options(&mut self, options: RenderOptions) {
        self.options = options;
    }

    /// 获取渲染选项引用
    pub fn options(&self) -> &RenderOptions {
        &self.options
    }

    /// 设置全局特效配置
    pub fn set_global_effect(&mut self, effect_config: Option<Arc<TextEffectConfig>>) {
        self.global_effect_config = effect_config;
    }

    // ===== 主渲染方法 =====

    /// 渲染所有弹幕到帧缓冲区
    ///
    /// barrages: 要渲染的弹幕列表
    /// emoji_mgr: Emoji 管理器引用
    /// global_effect: 全局特效配置（所有弹幕使用）
    ///
    /// 返回渲染的弹幕数量
    pub fn render(
        &mut self,
        barrages: &[&BarrageObject],
        emoji_mgr: &mut EmojiManager,
    ) -> u32 {
        let start = std::time::Instant::now();

        // 清空画布
        self.clear_buffer();

        let mut rendered_count = 0u32;
        let mut emoji_count = 0u32;

        for obj in barrages {
            if !obj.alive {
                continue;
            }

            // 获取弹幕当前透明度
            let alpha = obj.current_alpha(self.options.global_alpha);
            if alpha <= 0.001 {
                continue;
            }

            // 确定使用哪个特效配置
            let _effect_config: Option<&TextEffectConfig> = if self.options.enable_effects {
                if !self.options.use_global_effects {
                    obj.effect_config.as_deref()
                } else {
                    self.global_effect_config.as_deref()
                }
            } else {
                None
            };

            // 渲染弹幕内容（文本 + Emoji）
            if obj.segments.is_empty() {
                // 纯文本弹幕
                let (_tw, _th) = BitmapFont::measure_text(&obj.text, obj.font_size);
                let color = utils::rgba_from_u32(obj.color_rgba);
                // 调整透明度
                let adjusted_color = [color[0], color[1], color[2], (color[3] as f32 * alpha) as u8];

                // 渲染文本
                BitmapFont::render_text(
                    &mut self.frame_buffer,
                    &obj.text,
                    obj.x as i32,
                    obj.y as i32,
                    self.canvas_width,
                    self.canvas_height,
                    &adjusted_color,
                    obj.font_size,
                );

                rendered_count += 1;
            } else {
                // 图文混排弹幕
                let mut cursor_x = obj.x;

                for seg in &obj.segments {
                    if seg.is_emoji {
                        // 渲染 Emoji
                        if let Some(bitmap) = emoji_mgr.get_emoji_ref(&seg.emoji_id) {
                            let emoji_size = obj.font_size;
                            let _scale_x = emoji_size as f32 / bitmap.width as f32;
                            let _scale_y = emoji_size as f32 / bitmap.height as f32;

                            // 垂直居中
                            let emoji_y = obj.y + (obj.font_size as f32 - emoji_size as f32) / 2.0;

                            self.blit_emoji(
                                &bitmap.rgba_data,
                                bitmap.width,
                                bitmap.height,
                                cursor_x as i32,
                                emoji_y as i32,
                                emoji_size,
                                emoji_size,
                                alpha,
                            );

                            cursor_x += emoji_size as f32 + 2.0; // 间距
                            emoji_count += 1;
                        }
                    } else {
                        // 渲染文本片段
                        let (tw, _th) = BitmapFont::measure_text(&seg.text, obj.font_size);
                        let color = utils::rgba_from_u32(obj.color_rgba);
                        let adjusted_color = [color[0], color[1], color[2], (color[3] as f32 * alpha) as u8];

                        BitmapFont::render_text(
                            &mut self.frame_buffer,
                            &seg.text,
                            cursor_x as i32,
                            obj.y as i32,
                            self.canvas_width,
                            self.canvas_height,
                            &adjusted_color,
                            obj.font_size,
                        );

                        cursor_x += tw as f32 + 2.0;
                    }
                }

                rendered_count += 1;
            }
        }

        self.stats.barrages_rendered = rendered_count;
        self.stats.emojis_rendered = emoji_count;
        self.stats.render_time_us = start.elapsed().as_micros() as u64;

        rendered_count
    }

    /// 渲染单个弹幕并返回其 RGBA 缓冲区
    ///
    /// 用于预先渲染弹幕然后进行特效处理
    pub fn render_barrage_to_buffer(
        &self,
        obj: &BarrageObject,
        _emoji_mgr: &mut EmojiManager,
    ) -> Option<Vec<u8>> {
        if !obj.alive {
            return None;
        }

        let (tw, th) = BitmapFont::measure_text(&obj.text, obj.font_size);
        let buffer_size = (tw * th * 4) as usize;
        if buffer_size == 0 || buffer_size > 4 * 1024 * 1024 {
            return None; // 超过 4MB 不处理
        }

        let mut buffer = vec![0u8; buffer_size];
        let color = utils::rgba_from_u32(obj.color_rgba);

        BitmapFont::render_text(
            &mut buffer,
            &obj.text,
            0,
            0,
            tw,
            th,
            &color,
            obj.font_size,
        );

        Some(buffer)
    }

    // ===== Emoji 混合渲染 =====

    /// 将 Emoji 位图 blit 到帧缓冲区
    fn blit_emoji(
        &mut self,
        rgba_data: &[u8],
        src_w: u32,
        src_h: u32,
        dst_x: i32,
        dst_y: i32,
        dst_w: u32,
        dst_h: u32,
        alpha: f32,
    ) {
        // 计算缩放
        let scale_x = src_w as f32 / dst_w as f32;
        let scale_y = src_h as f32 / dst_h as f32;

        for dy in 0..dst_h {
            let py = dst_y + dy as i32;
            if py < 0 || py >= self.canvas_height as i32 {
                continue;
            }

            let src_y = (dy as f32 * scale_y) as u32;

            for dx in 0..dst_w {
                let px = dst_x + dx as i32;
                if px < 0 || px >= self.canvas_width as i32 {
                    continue;
                }

                let src_x = (dx as f32 * scale_x) as u32;
                let src_idx = ((src_y * src_w + src_x) * 4) as usize;

                if src_idx + 3 >= rgba_data.len() {
                    continue;
                }

                let fg_pixel = [
                    rgba_data[src_idx],
                    rgba_data[src_idx + 1],
                    rgba_data[src_idx + 2],
                    (rgba_data[src_idx + 3] as f32 * alpha) as u8,
                ];

                if fg_pixel[3] == 0 {
                    continue; // 透明像素跳过
                }

                let dst_idx = ((py as u32 * self.canvas_width + px as u32) * 4) as usize;
                if dst_idx + 3 >= self.frame_buffer.len() {
                    continue;
                }

                let bg_pixel = [
                    self.frame_buffer[dst_idx],
                    self.frame_buffer[dst_idx + 1],
                    self.frame_buffer[dst_idx + 2],
                    self.frame_buffer[dst_idx + 3],
                ];

                let blended = utils::blend_alpha_fast(&bg_pixel, &fg_pixel);
                self.frame_buffer[dst_idx] = blended[0];
                self.frame_buffer[dst_idx + 1] = blended[1];
                self.frame_buffer[dst_idx + 2] = blended[2];
                self.frame_buffer[dst_idx + 3] = blended[3];
            }
        }
    }

    // ===== 工具方法 =====

    /// 获取画布尺寸
    pub fn dimensions(&self) -> (u32, u32) {
        (self.canvas_width, self.canvas_height)
    }

    /// 拷贝当前帧缓冲区到外部切片
    pub fn copy_frame(&self, buffer: &mut [u8]) -> bool {
        if buffer.len() < self.frame_buffer.len() {
            return false;
        }
        let len = buffer.len().min(self.frame_buffer.len());
        buffer[..len].copy_from_slice(&self.frame_buffer[..len]);
        true
    }

    /// 重置渲染统计
    pub fn reset_stats(&mut self) {
        self.stats = RenderStats::default();
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new(1920, 1080);
        assert_eq!(renderer.dimensions(), (1920, 1080));
        assert_eq!(renderer.frame_buffer().len(), 1920 * 1080 * 4);
    }

    #[test]
    fn test_clear_buffer() {
        let mut renderer = Renderer::with_background(100, 100, 0x00000000);
        renderer.clear_buffer();
        // 所有像素应全透明
        assert_eq!(renderer.frame_buffer()[0], 0);
        assert_eq!(renderer.frame_buffer()[1], 0);
        assert_eq!(renderer.frame_buffer()[2], 0);
        assert_eq!(renderer.frame_buffer()[3], 0);
    }

    #[test]
    fn test_clear_buffer_with_bg() {
        let mut renderer = Renderer::with_background(100, 100, 0xFF0000FF); // 红色背景
        renderer.clear_buffer();
        assert_eq!(renderer.frame_buffer()[0], 0xFF);
        assert_eq!(renderer.frame_buffer()[3], 0xFF);
    }

    #[test]
    fn test_resize() {
        let mut renderer = Renderer::new(100, 100);
        renderer.resize(200, 200);
        assert_eq!(renderer.dimensions(), (200, 200));
        assert_eq!(renderer.frame_buffer().len(), 200 * 200 * 4);
    }

    #[test]
    fn test_copy_frame() {
        let renderer = Renderer::new(10, 10);
        let mut buffer = vec![0u8; 10 * 10 * 4];
        assert!(renderer.copy_frame(&mut buffer));
    }

    #[test]
    fn test_copy_frame_insufficient() {
        let renderer = Renderer::new(100, 100);
        let mut buffer = vec![0u8; 10]; // 太小
        assert!(!renderer.copy_frame(&mut buffer));
    }
}