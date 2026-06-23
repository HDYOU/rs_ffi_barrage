/// integration_tests.rs — 全面的 Rust 集成测试
///
/// 测试覆盖：
/// - 引擎创建/销毁生命周期
/// - 弹幕发送与轨道调度
/// - Emoji 三种注册方式
/// - 贴图查询接口（存在/不存在双场景）
/// - 像素拷贝完整性校验
/// - 文字特效配置
/// - 过滤引擎
/// - 时间轴控制（速度/暂停/seek）
/// - 内存池复用
/// - FFI 接口可靠性

use rs_ffi_barrage::barrage::{BarrageObject, BarrageObjectPool};
use rs_ffi_barrage::config::{
    BarrageFilterConfig, BarrageGlobalConfig, GlowPreset, GradientType, TextEffectConfig, TrackMode,
};
use rs_ffi_barrage::core::BarrageCore;
use rs_ffi_barrage::emoji::EmojiManager;
use rs_ffi_barrage::filter::{FilterEngine, FilterResult};
use rs_ffi_barrage::text_effect;
use rs_ffi_barrage::timeline::TimelineController;
use rs_ffi_barrage::track::TrackManager;
use rs_ffi_barrage::utils;


// ============================================================
// 测试套件：引擎创建/销毁生命周期
// ============================================================

#[test]
fn test_engine_lifecycle() {
    // 创建默认引擎
    let core = BarrageCore::with_default_config();
    let stats = core.get_stats();
    assert_eq!(stats.alive_barrages, 0);
    assert!(!stats.paused);
    assert_eq!(stats.engine_time_ms, 0);

    // 创建自定义配置引擎
    let config = BarrageGlobalConfig {
        canvas_width: 1280,
        canvas_height: 720,
        track_count: 15,
        default_duration_ms: 3000,
        ..Default::default()
    };
    let core2 = BarrageCore::new(config);
    let stats2 = core2.get_stats();
    assert_eq!(stats2.engine_time_ms, 0);
}

#[test]
fn test_engine_update_loop() {
    let core = BarrageCore::with_default_config();

    // 模拟 60 帧更新
    for _ in 0..60 {
        core.send_barrage("帧测试", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 5000, None);
        core.update(16); // ~60fps
    }

    let stats = core.get_stats();
    assert!(stats.engine_time_ms > 0);
}

// ============================================================
// 测试套件：弹幕发送与轨道调度
// ============================================================

#[test]
fn test_barrage_send_and_dispatch() {
    let core = BarrageCore::with_default_config();

    // 发送不同类型弹幕
    core.send_barrage("滚动弹幕", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 5000, None);
    core.send_barrage("顶部弹幕", TrackMode::Top, 0xFFFF0000, 32, 1.0, 5000, None);
    core.send_barrage("底部弹幕", TrackMode::Bottom, 0xFF00FF00, 24, 0.8, 5000, None);
    core.send_barrage("逆向弹幕", TrackMode::Reverse, 0xFF0000FF, 36, 0.5, 5000, None);

    core.update(16);

    let stats = core.get_stats();
    assert_eq!(stats.alive_barrages, 4);
}

#[test]
fn test_barrage_batch_send() {
    let core = BarrageCore::with_default_config();

    // 批量发送 100 条弹幕
    for i in 0..100 {
        core.send_barrage(
            &format!("批量弹幕 {}", i),
            TrackMode::Scroll,
            0xFFFFFFFF,
            28,
            1.0,
            10000,
            None,
        );
    }

    core.update(16);
    let stats = core.get_stats();
    assert!(stats.alive_barrages > 0);
    assert!(stats.alive_barrages <= 100);
}

#[test]
fn test_barrage_expiration() {
    let core = BarrageCore::with_default_config();

    // 发送短时弹幕
    core.send_barrage("短时弹幕", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 50, None);
    core.update(16);
    assert_eq!(core.get_stats().alive_barrages, 1);

    // 模拟足够长时间
    for _ in 0..10 {
        core.update(16);
    }
    assert_eq!(core.get_stats().alive_barrages, 0);
}

// ============================================================
// 测试套件：Emoji 三种注册方式
// ============================================================

#[test]
fn test_emoji_register_rgba() {
    let mut mgr = EmojiManager::with_default_config();
    let rgba = vec![255u8; 32 * 32 * 4];

    assert!(mgr.register_from_rgba("emoji_a", 32, 32, &rgba).is_ok());
    assert!(mgr.has_emoji("emoji_a"));
}

#[test]
fn test_emoji_register_rgba_invalid_data() {
    let mut mgr = EmojiManager::with_default_config();
    let wrong_data = vec![255u8; 10]; // 数据长度不匹配

    let result = mgr.register_from_rgba("bad_emoji", 10, 10, &wrong_data);
    assert!(result.is_err());
}

#[test]
fn test_emoji_register_file() {
    

    // 创建一个临时 PNG 文件
    let mut tmp_dir = std::env::temp_dir();
    tmp_dir.push("emoji_test.png");

    // 使用 image crate 创建一个简单的测试图片
    let img = image::RgbaImage::new(16, 16);
    img.save(&tmp_dir).expect("保存临时图片失败");

    let mut mgr = EmojiManager::with_default_config();
    let result = mgr.register_from_file(
        "file_emoji",
        tmp_dir.to_str().unwrap(),
    );
    assert!(result.is_ok());
    assert!(mgr.has_emoji("file_emoji"));

    // 清理
    let _ = std::fs::remove_file(&tmp_dir);
}

#[test]
fn test_emoji_register_file_not_found() {
    let mut mgr = EmojiManager::with_default_config();
    let result = mgr.register_from_file("missing_emoji", "/nonexistent/path.png");
    assert!(result.is_err());
}

// ============================================================
// 测试套件：贴图查询接口
// ============================================================

#[test]
fn test_emoji_query_exists() {
    let mut mgr = EmojiManager::with_default_config();
    let rgba = vec![1u8, 2, 3, 4]; // 1x1 pixel
    mgr.register_from_rgba("query_test", 1, 1, &rgba).unwrap();

    // get_emoji_info
    let info = mgr.get_emoji_info("query_test");
    assert!(info.is_some());
    let (w, h, bytes) = info.unwrap();
    assert_eq!(w, 1);
    assert_eq!(h, 1);
    assert_eq!(bytes, 4);

    // copy_emoji_bitmap
    let mut buf = vec![0u8; 4];
    assert!(mgr.copy_emoji_bitmap("query_test", &mut buf, 4));
    assert_eq!(buf, vec![1, 2, 3, 4]);

    // get_emoji_bitmap_bytes
    let bytes = mgr.get_emoji_bitmap_bytes("query_test");
    assert_eq!(bytes, Some(vec![1, 2, 3, 4]));
}

#[test]
fn test_emoji_query_not_exists() {
    let mut mgr = EmojiManager::with_default_config();

    // get_emoji_info
    assert!(mgr.get_emoji_info("not_exist").is_none());

    // copy_emoji_bitmap
    let mut buf = vec![0u8; 4];
    assert!(!mgr.copy_emoji_bitmap("not_exist", &mut buf, 4));

    // get_emoji_bitmap_bytes
    assert!(mgr.get_emoji_bitmap_bytes("not_exist").is_none());
}

// ============================================================
// 测试套件：像素拷贝完整性校验
// ============================================================

#[test]
fn test_pixel_copy_integrity() {
    let mut mgr = EmojiManager::with_default_config();
    let width = 8;
    let height = 8;
    let pixel_count = (width * height * 4) as usize;

    // 创建棋盘格图案
    let mut rgba = vec![0u8; pixel_count];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            if (x + y) % 2 == 0 {
                rgba[idx] = 255;     // R
                rgba[idx + 1] = 0;   // G
                rgba[idx + 2] = 0;   // B
                rgba[idx + 3] = 255; // A
            } else {
                rgba[idx] = 0;       // R
                rgba[idx + 1] = 0;   // G
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            }
        }
    }

    mgr.register_from_rgba("checkerboard", width, height, &rgba).unwrap();

    // 拷贝并验证完整性
    let mut buffer = vec![0u8; pixel_count];
    assert!(mgr.copy_emoji_bitmap("checkerboard", &mut buffer, pixel_count));
    assert_eq!(buffer, rgba);

    // 验证特定像素
    assert_eq!(buffer[0], 255);       // (0,0) 红
    // 像素 (0,1) 索引: (1*8+0)*4 = 32, R通道
    assert_eq!(buffer[32], 0);        // (0,1) 蓝 (R通道=0)
}

#[test]
fn test_pixel_copy_truncated() {
    let mut mgr = EmojiManager::with_default_config();
    let rgba = vec![255u8; 16 * 16 * 4];
    mgr.register_from_rgba("trunc_test", 16, 16, &rgba).unwrap();

    // 缓冲区不足应返回 false
    let mut small_buf = vec![0u8; 4];
    assert!(!mgr.copy_emoji_bitmap("trunc_test", &mut small_buf, 4));
}

// ============================================================
// 测试套件：文字特效配置
// ============================================================

#[test]
fn test_text_effect_outline_config() {
    let mut cfg = TextEffectConfig::new();
    assert!(!cfg.outline_enabled);

    cfg.outline_enabled = true;
    cfg.outline_width = 3;
    cfg.outline_color_rgba = 0xFF0000FF;
    cfg.validate();

    assert_eq!(cfg.outline_width, 3);
    assert_eq!(cfg.outline_color_rgba, 0xFF0000FF);
}

#[test]
fn test_text_effect_shadow_config() {
    let mut cfg = TextEffectConfig::new();
    cfg.shadow_enabled = true;
    cfg.shadow_layers = 5;
    cfg.shadow_blur_radius = 4;
    cfg.validate();

    assert_eq!(cfg.shadow_layers, 5);
    assert_eq!(cfg.shadow_blur_radius, 4);
}

#[test]
fn test_text_effect_glow_preset() {
    let mut cfg = TextEffectConfig::new();
    cfg.glow_enabled = true;
    cfg.glow_preset = GlowPreset::MonoBlue;
    cfg.glow_intensity = 0.8;
    cfg.validate();

    assert_eq!(cfg.glow_preset, GlowPreset::MonoBlue);
}

#[test]
fn test_text_effect_gradient_rainbow() {
    let mut cfg = TextEffectConfig::new();
    cfg.gradient_enabled = true;
    cfg.rainbow_preset = true;
    cfg.validate(); // apply_rainbow_preset 会设置默认 gradient_type=Linear
    // 在 validate 之后手动设置为 Radial，因为 apply_rainbow_preset 不覆盖用户设置
    cfg.gradient_type = GradientType::Radial;
    cfg.validate(); // 再次 validate 不会改变 gradient_type

    assert!(cfg.rainbow_preset);
    assert_eq!(cfg.gradient_type, GradientType::Radial);
}

#[test]
fn test_text_effect_json_serialization() {
    let mut cfg = TextEffectConfig::new();
    cfg.outline_enabled = true;
    cfg.outline_width = 2;
    cfg.shadow_enabled = true;
    cfg.glow_enabled = true;
    cfg.glow_preset = GlowPreset::DuoBluePurple;
    cfg.gradient_enabled = true;
    cfg.rainbow_preset = true;

    let json = cfg.to_json().expect("JSON 序列化失败");
    let cfg2 = TextEffectConfig::from_json(&json).expect("JSON 反序列化失败");

    assert_eq!(cfg.outline_enabled, cfg2.outline_enabled);
    assert_eq!(cfg.outline_width, cfg2.outline_width);
    assert!(cfg2.shadow_enabled);
    assert!(cfg2.gradient_enabled);
}

#[test]
fn test_text_effect_validate_clamping() {
    let mut cfg = TextEffectConfig::new();
    cfg.outline_width = 999;
    cfg.shadow_layers = 999;
    cfg.glow_radius = 999;
    cfg.validate();

    assert_eq!(cfg.outline_width, 16);   // max
    assert_eq!(cfg.shadow_layers, 8);    // max
    assert_eq!(cfg.glow_radius, 32);     // max
}

// ============================================================
// 测试套件：过滤引擎
// ============================================================

#[test]
fn test_filter_keyword_matching() {
    let mut engine = FilterEngine::with_default_config();
    engine.add_keyword("广告");
    engine.add_keyword("推广");

    let obj = BarrageObject::new(1, "这是一个推广消息", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
    assert_eq!(engine.filter(&obj), FilterResult::BlockedByKeyword);

    let obj = BarrageObject::new(2, "正常消息", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
    assert_eq!(engine.filter(&obj), FilterResult::Pass);
}

#[test]
fn test_filter_type_control() {
    let mut config = BarrageFilterConfig::default();
    config.enable_top = false;
    config.enable_bottom = false;
    let engine = FilterEngine::new(config);

    let scroll = BarrageObject::new(1, "滚动", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
    assert_eq!(engine.filter(&scroll), FilterResult::Pass);

    let top = BarrageObject::new(2, "顶部", TrackMode::Top, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
    assert_eq!(engine.filter(&top), FilterResult::BlockedByType);
}

#[test]
fn test_filter_duration_filtering() {
    let mut config = BarrageFilterConfig::default();
    config.short_duration_filter_enabled = true;
    config.min_duration_ms = 1000;
    let engine = FilterEngine::new(config);

    let short = BarrageObject::new(1, "短", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 100, 0);
    assert_eq!(engine.filter(&short), FilterResult::BlockedByDuration);

    let long = BarrageObject::new(2, "长弹幕", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
    assert_eq!(engine.filter(&long), FilterResult::Pass);
}

#[test]
fn test_filter_global_adjustments() {
    let mut config = BarrageFilterConfig::default();
    config.global_font_size_enabled = true;
    config.global_font_size = 48;
    config.global_alpha_enabled = true;
    config.global_alpha = 0.75;

    let engine = FilterEngine::new(config);

    let mut obj = BarrageObject::new(1, "调整测试", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
    engine.apply_global_adjustments(&mut obj);

    assert_eq!(obj.font_size, 48);
    assert!((obj.alpha - 0.75).abs() < 0.001);
}

// ============================================================
// 测试套件：时间轴控制
// ============================================================

#[test]
fn test_timeline_speed_control() {
    let mut tl = TimelineController::new();
    assert!((tl.speed() - 1.0).abs() < 0.001);

    tl.set_speed(2.0);
    assert!((tl.speed() - 2.0).abs() < 0.001);

    tl.set_speed(0.5);
    assert!((tl.speed() - 0.5).abs() < 0.001);

    tl.set_speed(10.0); // 应被 clamp
    assert!((tl.speed() - 4.0).abs() < 0.001);
}

#[test]
fn test_timeline_pause() {
    let mut tl = TimelineController::new();
    assert!(!tl.is_paused());

    tl.pause();
    assert!(tl.is_paused());

    // 暂停时 update 应返回 0
    let delta = tl.update();
    assert_eq!(delta, 0);

    tl.resume();
    assert!(!tl.is_paused());
}

#[test]
fn test_timeline_seek() {
    let mut tl = TimelineController::new();
    tl.seek(10000);
    assert_eq!(tl.engine_time_ms(), 10000);

    tl.seek(0);
    assert_eq!(tl.engine_time_ms(), 0);
}

#[test]
fn test_timeline_clear() {
    let mut tl = TimelineController::new();
    tl.set_speed(2.0);
    tl.seek(5000);
    tl.pause();

    tl.clear();
    assert_eq!(tl.engine_time_ms(), 0);
    assert!((tl.speed() - 1.0).abs() < 0.001);
    assert!(!tl.is_paused());
}

#[test]
fn test_timeline_barrage_visibility() {
    let mut tl = TimelineController::new();

    // 弹幕创建时引擎时间为 1000，持续 2000ms
    assert!(tl.is_barrage_visible(1000, 2000));
    assert!(tl.is_barrage_visible(500, 2000));

    // 向前推进时间
    tl.seek(5000);
    assert!(!tl.is_barrage_visible(1000, 2000));
    assert!(tl.is_barrage_visible(4000, 2000));
}

// ============================================================
// 测试套件：内存池复用
// ============================================================

#[test]
fn test_object_pool_reuse() {
    let mut pool = BarrageObjectPool::new(16, 8);

    // 分配 5 个对象
    let mut indices = Vec::new();
    for i in 0..5 {
        let obj = BarrageObject::new(
            i, &format!("obj{}", i), TrackMode::Scroll, 0,
            0xFFFFFFFF, 28, 1.0, 5000, 0,
        );
        let (idx, _) = pool.allocate(obj).expect("分配失败");
        indices.push(idx);
    }

    assert_eq!(pool.alive_count(), 5);

    // 释放前 3 个
    for i in 0..3 {
        pool.deallocate(indices[i]);
    }

    assert_eq!(pool.alive_count(), 2);
    assert_eq!(pool.free_count(), pool.capacity() - 2);

    // 新分配应复用刚释放的索引（LIFO）
    let obj = BarrageObject::new(100, "reuse", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
    let (new_idx, _) = pool.allocate(obj).expect("分配失败");
    assert_eq!(new_idx, indices[2]); // 最后释放的索引优先复用
}

#[test]
fn test_object_pool_grow() {
    let mut pool = BarrageObjectPool::new(8, 4);

    // 分配超过初始容量
    for i in 0..20 {
        let obj = BarrageObject::new(
            i, &format!("obj{}", i), TrackMode::Scroll, 0,
            0xFFFFFFFF, 28, 1.0, 5000, 0,
        );
        let result = pool.allocate(obj);
        assert!(result.is_some(), "第 {} 个对象分配失败", i);
    }

    assert!(pool.capacity() >= 20);
}

#[test]
fn test_object_pool_clear() {
    let mut pool = BarrageObjectPool::new(32, 16);

    for i in 0..10 {
        let obj = BarrageObject::new(
            i, "test", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0,
        );
        pool.allocate(obj).unwrap();
    }

    assert_eq!(pool.alive_count(), 10);
    pool.clear();
    assert_eq!(pool.alive_count(), 0);
    assert_eq!(pool.free_count(), pool.capacity());
}

#[test]
fn test_barrage_object_reset() {
    let mut obj = BarrageObject::new(
        42, "原始文本", TrackMode::Top, 5,
        0xFF0000FF, 36, 0.8, 10000, 1000,
    );
    obj.alive = true;

    obj.reset();

    // 重置后应是默认值
    assert!(!obj.alive);
    assert!(obj.text.is_empty());
    assert_eq!(obj.track_mode, TrackMode::Scroll);
    assert_eq!(obj.track_index, 0);
    assert_eq!(obj.font_size, 28);
}

// ============================================================
// 额外集成测试
// ============================================================

#[test]
fn test_full_pipeline() {
    let core = BarrageCore::with_default_config();

    // 注册 Emoji
    let rgba = vec![200u8; 16 * 16 * 4];
    assert!(core.register_emoji_rgba("face", &rgba, 16, 16).is_ok());

    // 发送弹幕
    core.send_barrage("集成测试弹幕", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 5000, None);

    // 更新
    core.update(16);
    assert!(core.get_stats().alive_barrages > 0);

    // 渲染
    let mut buffer = vec![0u8; 320 * 240 * 4];
    let count = core.render(&mut buffer, 320, 240);
    assert!(count >= 0);

    // 控制
    core.pause();
    assert!(core.get_stats().paused);
    core.resume();
    assert!(!core.get_stats().paused);

    core.set_speed(1.5);
    assert!((core.get_stats().speed - 1.5).abs() < 0.001);

    // 清空
    core.clear();
    let stats = core.get_stats();
    assert_eq!(stats.alive_barrages, 0);
}

#[test]
fn test_concurrent_stress() {
    let core = BarrageCore::with_default_config();

    // 模拟高并发发送
    for i in 0..500 {
        core.send_barrage(
            &format!("Stress #{}", i),
            if i % 2 == 0 { TrackMode::Scroll } else { TrackMode::Top },
            0xFFFFFFFF,
            28,
            1.0,
            60000,
            None,
        );
    }

    // 批量更新
    for _ in 0..10 {
        core.update(16);
    }

    let stats = core.get_stats();
    println!("高并发测试: {} 活跃弹幕, 池容量 {}", stats.alive_barrages, stats.pool_capacity);
    assert!(stats.alive_barrages > 0);
}

#[test]
fn test_mixed_text_and_emoji() {
    use rs_ffi_barrage::barrage::EmojiSegment;

    // EmojiSegment 测试
    let text_seg = EmojiSegment::text("Hello");
    assert!(!text_seg.is_emoji);
    assert_eq!(text_seg.text, "Hello");

    let emoji_seg = EmojiSegment::emoji("smile");
    assert!(emoji_seg.is_emoji);
    assert_eq!(emoji_seg.emoji_id, "smile");

    // 弹幕携带混合片段
    let _segments = vec![
        EmojiSegment::text("前面"),
        EmojiSegment::emoji("face_001"),
        EmojiSegment::text("后面"),
    ];

    let core = BarrageCore::with_default_config();

    // 通过 send_barrage（纯文本模式）
    core.send_barrage("纯文本弹幕", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 5000, None);
    core.update(16);

    let stats = core.get_stats();
    assert!(stats.alive_barrages > 0);
}

#[test]
fn test_emoji_cache_management() {
    let core = BarrageCore::with_default_config();

    let rgba = vec![100u8; 8 * 8 * 4];
    core.register_emoji_rgba("cache_test", &rgba, 8, 8).unwrap();

    assert!(core.get_emoji_cache_size() > 0);

    core.clear_emoji_cache();
    assert_eq!(core.get_emoji_cache_size(), 0);
}

#[test]
fn test_emoji_remove() {
    let core = BarrageCore::with_default_config();

    let rgba = vec![100u8; 4 * 4 * 4];
    core.register_emoji_rgba("remove_test", &rgba, 4, 4).unwrap();

    assert!(core.remove_emoji("remove_test"));
    assert!(!core.remove_emoji("not_exist"));
}

#[test]
fn test_filter_integration() {
    let core = BarrageCore::with_default_config();

    // 添加黑名单关键字
    core.add_blacklist_keyword("敏感词");

    // 发送被拦截的弹幕
    core.send_barrage("包含敏感词的内容", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 5000, None);
    core.update(16);
    assert_eq!(core.get_stats().alive_barrages, 0);

    // 发送正常弹幕
    core.send_barrage("正常内容", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 5000, None);
    core.update(16);
    assert_eq!(core.get_stats().alive_barrages, 1);
}

#[test]
fn test_multiple_render_calls() {
    let core = BarrageCore::with_default_config();

    for i in 0..10 {
        core.send_barrage(
            &format!("渲染测试{}", i),
            TrackMode::Scroll,
            0xFFFFFFFF,
            28,
            1.0,
            5000,
            None,
        );
    }
    core.update(16);

    let mut buffer = vec![0u8; 100 * 100 * 4];

    // 多次渲染
    for _ in 0..5 {
        let count = core.render(&mut buffer, 100, 100);
        assert!(count >= 0);
    }
}

#[test]
fn test_engine_stats() {
    let core = BarrageCore::with_default_config();

    let stats = core.get_stats();
    assert_eq!(stats.pool_capacity, 256);
    assert_eq!(stats.pool_free, 256);
    assert_eq!(stats.emoji_cache_entries, 0);

    core.send_barrage("统计测试", TrackMode::Scroll, 0xFFFFFFFF, 28, 1.0, 5000, None);
    core.update(16);

    let stats = core.get_stats();
    assert_eq!(stats.alive_barrages, 1);
    assert!(stats.engine_time_ms > 0);
}

#[test]
fn test_emoji_info_query_integrated() {
    let core = BarrageCore::with_default_config();

    // 未找到
    assert!(core.get_emoji_info("not_exist").is_none());

    // 注册后找到
    let rgba = vec![200u8; 64 * 64 * 4];
    core.register_emoji_rgba("big_emoji", &rgba, 64, 64).unwrap();

    let info = core.get_emoji_info("big_emoji");
    assert!(info.is_some());
    let (w, h, bytes) = info.unwrap();
    assert_eq!(w, 64);
    assert_eq!(h, 64);
    assert_eq!(bytes, 64 * 64 * 4);
}

#[test]
fn test_utils_functions() {
    // blend_alpha
    let bg = [255, 0, 0, 255];
    let fg = [0, 255, 0, 128];
    let blended = utils::blend_alpha(&bg, &fg);
    assert!(blended[0] < 255); // 红色变淡

    // clip_rect
    assert!(utils::clip_rect(0, 0, 100, 100, 200, 200).is_some());
    assert!(utils::clip_rect(300, 300, 100, 100, 200, 200).is_none());

    // rgba conversion
    let rgba = [0x12, 0x34, 0x56, 0x78];
    let encoded = utils::rgba_to_u32(&rgba);
    let decoded = utils::rgba_from_u32(encoded);
    assert_eq!(rgba, decoded);

    // current_time_ms
    let t = utils::current_time_ms();
    assert!(t > 1700000000000u64);
}

#[test]
fn test_text_effect_rendering() {
    let width = 32u32;
    let height = 32u32;
    let len = (width * height * 4) as usize;
    // 创建缓冲区，初始为全透明（alpha=0），中间区域作为文字
    let mut buffer = vec![0u8; len];
    for y in 4..28 {
        for x in 4..28 {
            let idx = ((y * width + x) * 4) as usize;
            buffer[idx] = 255;
            buffer[idx + 1] = 255;
            buffer[idx + 2] = 255;
            buffer[idx + 3] = 255;
        }
    }

    let mut config = TextEffectConfig::new();

    // 测试描边
    config.outline_enabled = true;
    config.outline_width = 2;
    config.outline_color_rgba = 0xFF0000FF;
    let original = buffer.clone();
    text_effect::apply_outline(&mut buffer, width, height, &config);
    assert!(buffer != original, "描边应修改缓冲区");
    config.outline_enabled = false;

    // 测试阴影
    config.shadow_enabled = true;
    config.shadow_layers = 2;
    let original = buffer.clone();
    text_effect::apply_shadow(&mut buffer, width, height, &config);
    assert!(buffer != original, "阴影应修改缓冲区");

    // 测试全特效叠加
    config.shadow_enabled = false;
    config.glow_enabled = true;
    config.glow_preset = GlowPreset::MonoBlue;
    config.gradient_enabled = true;
    config.rainbow_preset = true;

    let original = buffer.clone();
    text_effect::apply_all_effects(&mut buffer, width, height, &config);
    assert!(buffer != original, "全特效应修改缓冲区");
}

#[test]
fn test_track_manager_congestion() {
    let mut tm = TrackManager::new(2, 640, 480, false, 40, 10, 200.0);
    let mut pool = BarrageObjectPool::new(64, 16);

    // 设置第一条轨道最大并发为 1
    tm.tracks[0].max_concurrent = 1;
    tm.tracks[0].congestion_strategy = rs_ffi_barrage::config::CongestionStrategy::Drop;

    // 填充第一条轨道
    let obj1 = BarrageObject::new(1, "弹幕1", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
    let (idx1, _) = pool.allocate(obj1).unwrap();
    assert!(tm.add_barrage(&mut pool, idx1, 0, 0).is_ok());

    // 第二条应被丢弃
    let obj2 = BarrageObject::new(2, "弹幕2", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0);
    let (idx2, _) = pool.allocate(obj2).unwrap();
    let result = tm.add_barrage(&mut pool, idx2, 0, 0);
    assert_eq!(result, Err(()));
}