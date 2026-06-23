/// lib.rs — FFI C 接口层
///
/// 此文件导出所有 extern "C" 函数，供 Flutter/Dart 通过 dart:ffi 调用。
///
/// 每个 FFI 函数必须做：
/// - 空指针检查（null pointer check）
/// - 字符串长度校验（max length check）
/// - 数值范围 clamp
/// - 返回错误码
///
/// 所有 extern "C" 函数使用 #[no_mangle] 导出符号。

pub mod config;
pub mod barrage;
pub mod track;
pub mod timeline;
pub mod filter;
pub mod network;
pub mod emoji;
pub mod text_effect;
pub mod renderer;
pub mod core;
pub mod utils;

use std::os::raw::c_char;
use std::sync::Arc;

use crate::config::{
    BarrageFilterConfig, BarrageGlobalConfig, TextEffectConfig, TrackMode,
};
use crate::core::BarrageCore;

// ============================================================
// 内部辅助函数
// ============================================================

/// 从 C 字符串指针安全地创建 Rust 字符串引用
///
/// # 安全性
/// ptr 必须指向有效的 null 终止的 C 字符串，或为 NULL。
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    let len = utils::validate_c_str(ptr)?;
    if len > 65536 {
        return None; // 字符串过长
    }
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
    let s = std::str::from_utf8(bytes).ok()?;
    Some(s)
}

/// 从 C 字符串指针安全地创建 Rust String（拷贝）
unsafe fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    cstr_to_str(ptr).map(|s| s.to_string())
}

/// 获取引擎实例的可变引用
unsafe fn get_core_mut<'a>(ptr: *mut BarrageCore) -> Option<&'a mut BarrageCore> {
    if ptr.is_null() {
        return None;
    }
    Some(&mut *ptr)
}

/// 获取引擎实例的引用
unsafe fn get_core_ref<'a>(ptr: *const BarrageCore) -> Option<&'a BarrageCore> {
    if ptr.is_null() {
        return None;
    }
    Some(&*ptr)
}

// ============================================================
// 引擎生命周期
// ============================================================

/// 创建弹幕引擎实例
///
/// 使用默认配置创建。返回指向 BarrageCore 的裸指针。
/// 使用完毕后必须调用 rs_barrage_destroy 释放。
#[no_mangle]
pub extern "C" fn rs_barrage_create() -> *mut BarrageCore {
    let core = Box::new(BarrageCore::with_default_config());
    Box::into_raw(core)
}

/// 创建弹幕引擎实例（带全局配置）
///
/// config_ptr: 指向 BarrageGlobalConfig 的指针，可 NULL（使用默认配置）
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_create_with_config(
    config_ptr: *const BarrageGlobalConfig,
) -> *mut BarrageCore {
    let config = if config_ptr.is_null() {
        BarrageGlobalConfig::default()
    } else {
        let mut cfg = unsafe { (*config_ptr).clone() };
        cfg.validate();
        cfg
    };

    let core = Box::new(BarrageCore::new(config));
    Box::into_raw(core)
}

/// 销毁弹幕引擎实例
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_destroy(ptr: *mut BarrageCore) {
    if ptr.is_null() {
        return;
    }
    // 使用 drop 安全释放所有资源
    let _ = Box::from_raw(ptr);
}

/// 更新引擎状态（每帧调用）
///
/// delta_ms: 距上一帧的毫秒数
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_update(ptr: *mut BarrageCore, delta_ms: u64) {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return,
    };
    core.update(delta_ms);
}

/// 渲染当前帧到 RGBA 缓冲区
///
/// buffer: 输出缓冲区，大小必须为 width * height * 4 字节
/// width: 渲染宽度
/// height: 渲染高度
///
/// 返回: 渲染的弹幕数量；失败返回 -1
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_render(
    ptr: *mut BarrageCore,
    buffer: *mut u8,
    width: u32,
    height: u32,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -1,
    };

    if buffer.is_null() {
        return -1;
    }

    let expected_size = (width * height * 4) as usize;
    let buffer_slice = std::slice::from_raw_parts_mut(buffer, expected_size);

    core.render(buffer_slice, width, height)
}

// ============================================================
// 弹幕管理
// ============================================================

/// 发送弹幕
///
/// text: 弹幕文本（UTF-8 C 字符串）
/// track_type: 轨道类型（0=Scroll, 1=Top, 2=Bottom, 3=Reverse）
/// color_rgba: 颜色 RGBA
/// font_size: 字号（像素）
/// alpha: 透明度（0.0~1.0）
/// duration_ms: 持续时间（毫秒）
/// effect_json: 特效 JSON 字符串（NULL 表示使用全局特效）
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_send(
    ptr: *mut BarrageCore,
    text: *const c_char,
    track_type: i32,
    color_rgba: u32,
    font_size: u32,
    alpha: f32,
    duration_ms: u32,
    effect_json: *const c_char,
) {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return,
    };

    // 安全获取文本
    let text_str = match cstr_to_string(text) {
        Some(s) if !s.is_empty() => s,
        _ => return, // 空文本或无效指针，忽略
    };

    // 截断文本长度
    let text_str = if text_str.len() > 1024 {
        text_str[..1024].to_string()
    } else {
        text_str
    };

    // 校验轨道类型
    let track_mode = match track_type {
        0 => TrackMode::Scroll,
        1 => TrackMode::Top,
        2 => TrackMode::Bottom,
        3 => TrackMode::Reverse,
        _ => TrackMode::Scroll, // 默认滚动
    };

    // Clamp 数值
    let font_size = font_size.clamp(8, 120);
    let alpha = alpha.clamp(0.0, 1.0);
    let duration_ms = duration_ms.clamp(100, 60000);

    // 解析特效配置
    let effect_config: Option<Arc<TextEffectConfig>> = if !effect_json.is_null() {
        if let Some(json_str) = cstr_to_string(effect_json) {
            if !json_str.is_empty() {
                match TextEffectConfig::from_json(&json_str) {
                    Ok(cfg) => Some(Arc::new(cfg)),
                    Err(_) => None, // 解析失败，使用全局
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    core.send_barrage(
        &text_str,
        track_mode,
        color_rgba,
        font_size,
        alpha,
        duration_ms,
        effect_config,
    );
}

/// 清空所有弹幕
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_clear(ptr: *mut BarrageCore) {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return,
    };
    core.clear();
}

/// 设置播放速度
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_set_speed(ptr: *mut BarrageCore, speed: f32) {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return,
    };
    core.set_speed(speed);
}

/// 暂停播放
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_pause(ptr: *mut BarrageCore) {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return,
    };
    core.pause();
}

/// 恢复播放
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_resume(ptr: *mut BarrageCore) {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return,
    };
    core.resume();
}

/// Seek 跳转到指定时间点（毫秒）
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_seek(ptr: *mut BarrageCore, time_ms: u64) {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return,
    };
    core.seek(time_ms);
}

// ============================================================
// 全局特效配置
// ============================================================

/// 设置全局特效配置（传入 JSON 序列化配置）
///
/// effect_json: JSON 格式的特效配置字符串（NULL 清除全局特效）
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_set_global_effect(
    ptr: *mut BarrageCore,
    effect_json: *const c_char,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -1,
    };

    if effect_json.is_null() {
        core.set_global_effect(None);
        return 0;
    }

    let json_str = match cstr_to_string(effect_json) {
        Some(s) => s,
        None => return -1,
    };

    if json_str.is_empty() {
        core.set_global_effect(None);
        return 0;
    }

    match TextEffectConfig::from_json(&json_str) {
        Ok(cfg) => {
            let mut validated = cfg;
            validated.validate();
            core.set_global_effect(Some(Arc::new(validated)));
            0
        }
        Err(_) => -2, // JSON 解析失败
    }
}

// ============================================================
// Emoji 贴图注册
// ============================================================

/// 从 RGBA 数据注册 Emoji
///
/// id: Emoji 标识符（C 字符串）
/// rgba_data: RGBA 像素数据
/// width: 图像宽度
/// height: 图像高度
///
/// 返回: 0=成功, -1=参数错误, -2=注册失败
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_register_emoji_rgba(
    ptr: *mut BarrageCore,
    id: *const c_char,
    rgba_data: *const u8,
    width: u32,
    height: u32,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -1,
    };

    let id_str = match cstr_to_string(id) {
        Some(s) => s,
        None => return -1,
    };

    if rgba_data.is_null() || width == 0 || height == 0 {
        return -1;
    }

    let data_len = (width * height * 4) as usize;
    let data_slice = std::slice::from_raw_parts(rgba_data, data_len);

    match core.register_emoji_rgba(&id_str, data_slice, width, height) {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

/// 从文件注册 Emoji
///
/// id: Emoji 标识符（C 字符串）
/// file_path: 图片文件路径（C 字符串）
///
/// 返回: 0=成功, -1=参数错误, -2=加载失败
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_register_emoji_file(
    ptr: *mut BarrageCore,
    id: *const c_char,
    file_path: *const c_char,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -1,
    };

    let id_str = match cstr_to_string(id) {
        Some(s) => s,
        None => return -1,
    };

    let path_str = match cstr_to_string(file_path) {
        Some(s) => s,
        None => return -1,
    };

    match core.register_emoji_file(&id_str, &path_str) {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

/// 从 URL 注册 Emoji
///
/// id: Emoji 标识符（C 字符串）
/// url: 图片 URL（C 字符串）
///
/// 返回: 0=成功, -1=参数错误, -2=下载/加载失败
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_register_emoji_url(
    ptr: *mut BarrageCore,
    id: *const c_char,
    url: *const c_char,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -1,
    };

    let id_str = match cstr_to_string(id) {
        Some(s) => s,
        None => return -1,
    };

    let url_str = match cstr_to_string(url) {
        Some(s) => s,
        None => return -1,
    };

    match core.register_emoji_url(&id_str, &url_str) {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

// ============================================================
// Emoji 贴图查询（核心新接口）
// ============================================================

/// 获取 Emoji 信息
///
/// id: Emoji 标识符（C 字符串）
/// out_width: 输出宽度指针
/// out_height: 输出高度指针
/// out_bytes_len: 输出字节数指针
///
/// 返回: 0=找到, -1=未找到, -2=参数错误
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_get_emoji_info(
    ptr: *mut BarrageCore,
    id: *const c_char,
    out_width: *mut u32,
    out_height: *mut u32,
    out_bytes_len: *mut u64,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -2,
    };

    let id_str = match cstr_to_string(id) {
        Some(s) => s,
        None => return -2,
    };

    if out_width.is_null() || out_height.is_null() || out_bytes_len.is_null() {
        return -2;
    }

    match core.get_emoji_info(&id_str) {
        Some((w, h, bytes)) => {
            *out_width = w;
            *out_height = h;
            *out_bytes_len = bytes;
            0
        }
        None => -1,
    }
}

/// 拷贝 Emoji 像素到外部缓冲区
///
/// id: Emoji 标识符（C 字符串）
/// out_buffer: 输出缓冲区
/// buffer_len: 缓冲区长度
///
/// 返回: 0=成功, -1=未找到, -2=缓冲区不足, -3=参数错误
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_copy_emoji_bitmap(
    ptr: *mut BarrageCore,
    id: *const c_char,
    out_buffer: *mut u8,
    buffer_len: u64,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -3,
    };

    let id_str = match cstr_to_string(id) {
        Some(s) => s,
        None => return -3,
    };

    if out_buffer.is_null() || buffer_len == 0 {
        return -3;
    }

    let buffer_slice = std::slice::from_raw_parts_mut(out_buffer, buffer_len as usize);

    if core.copy_emoji_bitmap(&id_str, buffer_slice, buffer_len as usize) {
        0
    } else {
        // 区分未找到和缓冲区不足
        if core.get_emoji_info(&id_str).is_some() {
            -2 // 缓冲区不足
        } else {
            -1 // 未找到
        }
    }
}

// ============================================================
// 缓存管理
// ============================================================

/// 清除 Emoji 缓存
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_clear_emoji_cache(ptr: *mut BarrageCore) {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return,
    };
    core.clear_emoji_cache();
}

/// 移除指定 Emoji
///
/// 返回: 0=成功移除, -1=未找到, -2=参数错误
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_remove_emoji(
    ptr: *mut BarrageCore,
    id: *const c_char,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -2,
    };

    let id_str = match cstr_to_string(id) {
        Some(s) => s,
        None => return -2,
    };

    if core.remove_emoji(&id_str) {
        0
    } else {
        -1
    }
}

/// 获取缓存大小（字节数）
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_get_cache_size(ptr: *mut BarrageCore) -> u64 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return 0,
    };
    core.get_emoji_cache_size()
}

// ============================================================
// 过滤器
// ============================================================

/// 添加黑名单关键字
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_add_blacklist_keyword(
    ptr: *mut BarrageCore,
    keyword: *const c_char,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -1,
    };

    let keyword_str = match cstr_to_string(keyword) {
        Some(s) => s,
        None => return -1,
    };

    core.add_blacklist_keyword(&keyword_str);
    0
}

/// 设置过滤配置
///
/// config_ptr: 指向 BarrageFilterConfig 的指针
/// 返回: 0=成功, -1=参数错误
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_set_filter(
    ptr: *mut BarrageCore,
    config_ptr: *const BarrageFilterConfig,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -1,
    };

    if config_ptr.is_null() {
        return -1;
    }

    let config = (*config_ptr).clone();
    core.set_filter(config);
    0
}

// ============================================================
// 工具
// ============================================================

/// 获取引擎版本号
///
/// 返回静态 C 字符串，无需释放。
#[no_mangle]
pub extern "C" fn rs_barrage_version() -> *const c_char {
    let version = concat!("rs-barrage-engine/", env!("CARGO_PKG_VERSION"), "\0");
    version.as_ptr() as *const c_char
}

/// 获取引擎统计信息
///
/// stats_ptr: 输出统计信息的指针
/// 返回: 0=成功, -1=参数错误
#[no_mangle]
pub unsafe extern "C" fn rs_barrage_get_stats(
    ptr: *mut BarrageCore,
    stats_ptr: *mut crate::core::EngineStats,
) -> i32 {
    let core = match get_core_mut(ptr) {
        Some(c) => c,
        None => return -1,
    };

    if stats_ptr.is_null() {
        return -1;
    }

    *stats_ptr = core.get_stats();
    0
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_create_destroy() {
        let ptr = rs_barrage_create();
        assert!(!ptr.is_null());
        unsafe { rs_barrage_destroy(ptr); }
    }

    #[test]
    fn test_version() {
        let version = rs_barrage_version();
        assert!(!version.is_null());
        unsafe {
            let s = std::ffi::CStr::from_ptr(version);
            assert!(!s.to_str().unwrap().is_empty());
        }
    }

    #[test]
    fn test_send_barrage_ffi() {
        let ptr = rs_barrage_create();
        assert!(!ptr.is_null());

        unsafe {
            let text = CString::new("FFI测试弹幕").unwrap();
            rs_barrage_send(ptr, text.as_ptr(), 0, 0xFFFFFFFF, 28, 1.0, 5000, std::ptr::null());
            rs_barrage_update(ptr, 16);
        }

        unsafe { rs_barrage_destroy(ptr); }
    }

    #[test]
    fn test_pause_resume_ffi() {
        let ptr = rs_barrage_create();
        assert!(!ptr.is_null());

        unsafe {
            rs_barrage_pause(ptr);
            rs_barrage_resume(ptr);
            rs_barrage_set_speed(ptr, 2.0);
            rs_barrage_seek(ptr, 1000);
            rs_barrage_clear(ptr);
        }

        unsafe { rs_barrage_destroy(ptr); }
    }

    #[test]
    fn test_emoji_rgba_ffi() {
        let ptr = rs_barrage_create();
        assert!(!ptr.is_null());

        unsafe {
            let id = CString::new("ffi_emoji").unwrap();
            let rgba = vec![128u8; 16 * 16 * 4];
            let result = rs_barrage_register_emoji_rgba(
                ptr,
                id.as_ptr(),
                rgba.as_ptr(),
                16,
                16,
            );
            assert_eq!(result, 0);

            // 查询信息
            let mut w: u32 = 0;
            let mut h: u32 = 0;
            let mut bytes: u64 = 0;
            let info_result = rs_barrage_get_emoji_info(
                ptr,
                id.as_ptr(),
                &mut w,
                &mut h,
                &mut bytes,
            );
            assert_eq!(info_result, 0);
            assert_eq!(w, 16);
            assert_eq!(h, 16);

            // 拷贝像素
            let mut buffer = vec![0u8; 16 * 16 * 4];
            let copy_result = rs_barrage_copy_emoji_bitmap(
                ptr,
                id.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len() as u64,
            );
            assert_eq!(copy_result, 0);
        }

        unsafe { rs_barrage_destroy(ptr); }
    }

    #[test]
    fn test_emoji_not_found_ffi() {
        let ptr = rs_barrage_create();
        assert!(!ptr.is_null());

        unsafe {
            let id = CString::new("nonexistent").unwrap();
            let mut w: u32 = 0;
            let mut h: u32 = 0;
            let mut bytes: u64 = 0;
            let result = rs_barrage_get_emoji_info(
                ptr,
                id.as_ptr(),
                &mut w,
                &mut h,
                &mut bytes,
            );
            assert_eq!(result, -1);
        }

        unsafe { rs_barrage_destroy(ptr); }
    }

    #[test]
    fn test_render_ffi() {
        let ptr = rs_barrage_create();
        assert!(!ptr.is_null());

        unsafe {
            let text = CString::new("渲染FFI测试").unwrap();
            rs_barrage_send(ptr, text.as_ptr(), 0, 0xFFFFFFFF, 28, 1.0, 5000, std::ptr::null());
            rs_barrage_update(ptr, 16);

            let mut buffer = vec![0u8; 100 * 100 * 4];
            let count = rs_barrage_render(ptr, buffer.as_mut_ptr(), 100, 100);
            assert!(count >= 0);
        }

        unsafe { rs_barrage_destroy(ptr); }
    }

    #[test]
    fn test_global_effect_ffi() {
        let ptr = rs_barrage_create();
        assert!(!ptr.is_null());

        unsafe {
            let json = CString::new(r#"{"outline_enabled":true,"outline_width":2,"outline_color_rgba":255}"#).unwrap();
            let result = rs_barrage_set_global_effect(ptr, json.as_ptr());
            assert_eq!(result, 0);

            // 清除特效
            let result = rs_barrage_set_global_effect(ptr, std::ptr::null());
            assert_eq!(result, 0);
        }

        unsafe { rs_barrage_destroy(ptr); }
    }

    #[test]
    fn test_null_ptr_safety() {
        // 所有 FFI 函数应能安全处理 NULL 指针而不崩溃
        unsafe {
            rs_barrage_destroy(std::ptr::null_mut());
            rs_barrage_update(std::ptr::null_mut(), 16);
            rs_barrage_pause(std::ptr::null_mut());
            rs_barrage_resume(std::ptr::null_mut());
            rs_barrage_clear(std::ptr::null_mut());
            rs_barrage_set_speed(std::ptr::null_mut(), 1.0);
            rs_barrage_seek(std::ptr::null_mut(), 0);

            // render with null
            let result = rs_barrage_render(std::ptr::null_mut(), std::ptr::null_mut(), 100, 100);
            assert_eq!(result, -1);
        }
    }
}