/// barrage_bench.rs — Criterion 性能基准测试
///
/// 基准测试覆盖：
/// - 无特效纯文字弹幕渲染
/// - 单类特效弹幕渲染（描边/阴影/霓虹/渐变）
/// - 全特效叠加弹幕渲染
/// - 贴图查询接口耗时
/// - 高并发弹幕推送
/// - 帧更新耗时

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

use rs_ffi_barrage::config::{TextEffectConfig, GlowPreset, GradientType, TrackMode};
use rs_ffi_barrage::core::BarrageCore;
use rs_ffi_barrage::emoji::EmojiManager;
use rs_ffi_barrage::renderer::Renderer;
use rs_ffi_barrage::text_effect;

// ============================================================
// 辅助函数：创建测试缓冲区
// ============================================================

fn create_test_buffer(width: u32, height: u32) -> Vec<u8> {
    let len = (width * height * 4) as usize;
    let mut buf = vec![0u8; len];
    // 绘制一个矩形区域模拟文字
    for y in height / 4..3 * height / 4 {
        for x in width / 4..3 * width / 4 {
            let idx = ((y * width + x) * 4) as usize;
            buf[idx] = 255;
            buf[idx + 1] = 255;
            buf[idx + 2] = 255;
            buf[idx + 3] = 255;
        }
    }
    buf
}

// ============================================================
// 基准测试：无特效纯文字弹幕渲染
// ============================================================

fn bench_plain_barrage_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("plain_barrage_render");

    let core = BarrageCore::with_default_config();
    let mut buffer = vec![0u8; 640 * 480 * 4];

    // 预填弹幕
    for i in 0..50 {
        core.send_barrage(
            &format!("弹幕{}", i),
            TrackMode::Scroll,
            0xFFFFFFFF,
            28,
            1.0,
            60000,
            None,
        );
    }
    core.update(100); // 确保弹幕被处理

    group.bench_function("50_plain_barrages", |b| {
        b.iter(|| {
            let count = black_box(core.render(&mut buffer, 640, 480));
            black_box(count);
        })
    });

    group.finish();
}

// ============================================================
// 基准测试：单类特效弹幕
// ============================================================

fn bench_single_effect_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_effect_render");

    let core = BarrageCore::with_default_config();
    let mut buffer = vec![0u8; 640 * 480 * 4];

    // 描边特效
    {
        let mut effect = TextEffectConfig::new();
        effect.outline_enabled = true;
        effect.outline_width = 2;
        effect.outline_color_rgba = 0x000000FF;
        let effect_arc = std::sync::Arc::new(effect);

        for i in 0..30 {
            core.send_barrage(
                &format!("Outline{}", i),
                TrackMode::Scroll,
                0xFFFFFFFF,
                28,
                1.0,
                60000,
                Some(effect_arc.clone()),
            );
        }
    }

    core.update(100);

    group.bench_function("30_outline_barrages", |b| {
        b.iter(|| {
            let count = black_box(core.render(&mut buffer, 640, 480));
            black_box(count);
        })
    });

    group.finish();
}

// ============================================================
// 基准测试：全特效叠加
// ============================================================

fn bench_all_effects_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("all_effects_render");

    let core = BarrageCore::with_default_config();
    let mut buffer = vec![0u8; 640 * 480 * 4];

    // 全特效
    {
        let mut effect = TextEffectConfig::new();
        effect.outline_enabled = true;
        effect.outline_width = 2;
        effect.shadow_enabled = true;
        effect.shadow_layers = 3;
        effect.glow_enabled = true;
        effect.glow_preset = GlowPreset::MonoBlue;
        effect.glow_intensity = 0.6;
        effect.gradient_enabled = true;
        effect.rainbow_preset = true;
        let effect_arc = std::sync::Arc::new(effect);

        for i in 0..20 {
            core.send_barrage(
                &format!("AllFX{}", i),
                TrackMode::Scroll,
                0xFFFFFFFF,
                28,
                1.0,
                60000,
                Some(effect_arc.clone()),
            );
        }
    }

    core.update(100);

    group.bench_function("20_full_effects_barrages", |b| {
        b.iter(|| {
            let count = black_box(core.render(&mut buffer, 640, 480));
            black_box(count);
        })
    });

    group.finish();
}

// ============================================================
// 基准测试：贴图查询接口耗时
// ============================================================

fn bench_emoji_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("emoji_query");

    let mut mgr = EmojiManager::with_default_config();

    // 注册多个 Emoji
    for i in 0..50 {
        let rgba = vec![100u8; 32 * 32 * 4];
        let id = format!("emoji_{}", i);
        mgr.register_from_rgba(&id, 32, 32, &rgba).unwrap();
    }

    group.bench_function("get_emoji_info", |b| {
        b.iter(|| {
            for i in 0..50 {
                let id = format!("emoji_{}", i);
                let info = black_box(mgr.get_emoji_info(&id));
                black_box(info);
            }
        })
    });

    group.bench_function("copy_emoji_bitmap", |b| {
        let mut buffer = vec![0u8; 32 * 32 * 4];
        let buf_len = buffer.len();
        b.iter(|| {
            for i in 0..50 {
                let id = format!("emoji_{}", i);
                let result = black_box(mgr.copy_emoji_bitmap(&id, &mut buffer, buf_len));
                black_box(result);
            }
        })
    });

    group.bench_function("get_emoji_bitmap_bytes", |b| {
        b.iter(|| {
            for i in 0..50 {
                let id = format!("emoji_{}", i);
                let bytes = black_box(mgr.get_emoji_bitmap_bytes(&id));
                black_box(bytes);
            }
        })
    });

    // 未找到场景
    group.bench_function("get_emoji_info_not_found", |b| {
        b.iter(|| {
            let info = black_box(mgr.get_emoji_info("nonexistent_emoji"));
            black_box(info);
        })
    });

    group.finish();
}

// ============================================================
// 基准测试：高并发弹幕推送
// ============================================================

fn bench_high_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("high_throughput");

    let core = BarrageCore::with_default_config();

    // 批量发送 + 更新
    group.bench_function("send_1000_barrages_update_10", |b| {
        b.iter(|| {
            for i in 0..1000 {
                core.send_barrage(
                    &format!("吞吐测试{}", i),
                    TrackMode::Scroll,
                    0xFFFFFFFF,
                    28,
                    1.0,
                    60000,
                    None,
                );
            }
            for _ in 0..10 {
                core.update(black_box(16));
            }
        })
    });

    group.finish();
}

// ============================================================
// 基准测试：渲染缓冲区操作
// ============================================================

fn bench_render_buffer_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_buffer_ops");

    let renderer = Renderer::new(1920, 1080);
    let mut buffer = vec![0u8; 1920 * 1080 * 4];

    group.bench_function("copy_frame_1080p", |b| {
        b.iter(|| {
            let result = black_box(renderer.copy_frame(&mut buffer));
            black_box(result);
        })
    });

    group.finish();
}

// ============================================================
// 基准测试：对象池操作
// ============================================================

fn bench_object_pool_ops(c: &mut Criterion) {
    use rs_ffi_barrage::barrage::{BarrageObject, BarrageObjectPool};

    let mut group = c.benchmark_group("object_pool_ops");

    let mut pool = BarrageObjectPool::new(256, 64);

    group.bench_function("allocate_256_objects", |b| {
        b.iter(|| {
            for i in 0..256 {
                let obj = BarrageObject::new(
                    i, "bench", TrackMode::Scroll, 0, 0xFFFFFFFF, 28, 1.0, 5000, 0,
                );
                let result = black_box(pool.allocate(obj));
                black_box(result);
            }
        })
    });

    group.finish();
}

// ============================================================
// 基准测试：文字特效算法
// ============================================================

fn bench_text_effect_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_effect_algorithms");

    let width = 128u32;
    let height = 64u32;
    let mut buffer = create_test_buffer(width, height);

    let mut config = TextEffectConfig::new();

    // 描边
    config.outline_enabled = true;
    config.outline_width = 3;
    let mut buf = buffer.clone();
    group.bench_function("outline_128x64", |b| {
        b.iter(|| {
            text_effect::apply_outline(black_box(&mut buf), width, height, &config);
            black_box(&buf);
        })
    });
    config.outline_enabled = false;

    // 阴影
    config.shadow_enabled = true;
    config.shadow_layers = 3;
    config.shadow_blur_radius = 2;
    let mut buf = buffer.clone();
    group.bench_function("shadow_128x64", |b| {
        b.iter(|| {
            text_effect::apply_shadow(black_box(&mut buf), width, height, &config);
            black_box(&buf);
        })
    });
    config.shadow_enabled = false;

    // 霓虹
    config.glow_enabled = true;
    config.glow_preset = GlowPreset::MonoBlue;
    config.glow_radius = 6;
    let mut buf = buffer.clone();
    group.bench_function("glow_128x64", |b| {
        b.iter(|| {
            text_effect::apply_glow(black_box(&mut buf), width, height, &config);
            black_box(&buf);
        })
    });
    config.glow_enabled = false;

    // 渐变
    config.gradient_enabled = true;
    config.rainbow_preset = true;
    let mut buf = buffer.clone();
    group.bench_function("gradient_128x64", |b| {
        b.iter(|| {
            text_effect::apply_gradient(black_box(&mut buf), width, height, &config);
            black_box(&buf);
        })
    });
    config.gradient_enabled = false;

    // 全特效
    config.outline_enabled = true;
    config.shadow_enabled = true;
    config.glow_enabled = true;
    config.glow_preset = GlowPreset::DuoBluePurple;
    config.gradient_enabled = true;
    config.rainbow_preset = true;
    let mut buf = buffer.clone();
    group.bench_function("all_effects_128x64", |b| {
        b.iter(|| {
            text_effect::apply_all_effects(black_box(&mut buf), width, height, &config);
            black_box(&buf);
        })
    });

    group.finish();
}

// ============================================================
// 基准测试：引擎帧更新延迟
// ============================================================

fn bench_engine_frame_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_frame_update");

    let core = BarrageCore::with_default_config();

    // 预填充弹幕
    for i in 0..200 {
        core.send_barrage(
            &format!("FrameTest{}", i),
            TrackMode::Scroll,
            0xFFFFFFFF,
            28,
            1.0,
            60000,
            None,
        );
    }
    core.update(16);

    group.bench_function("update_one_frame", |b| {
        b.iter(|| {
            black_box(core.update(16));
        })
    });

    group.finish();
}

// ============================================================
// 基准测试组注册
// ============================================================

criterion_group!(
    benches,
    bench_plain_barrage_render,
    bench_single_effect_render,
    bench_all_effects_render,
    bench_emoji_query,
    bench_high_throughput,
    bench_render_buffer_ops,
    bench_object_pool_ops,
    bench_text_effect_algorithms,
    bench_engine_frame_update,
);
criterion_main!(benches);