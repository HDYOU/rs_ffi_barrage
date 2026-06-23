/**
 * =============================================================================
 * rs_ffi_barrage — C 语言 FFI Demo 程序
 *
 * 脱离 Flutter 环境，直接链接 librs_ffi_barrage 动态库，
 * 验证核心 C ABI 接口的可用性。
 *
 * 编译:
 *   gcc main.c -o rs_ffi_barrage_demo -lrs_ffi_barrage -L/path/to/lib
 *
 * 功能演示流程:
 *   1. 创建引擎
 *   2. 注册一个 16x16 红色/蓝色 RGBA 测试贴图
 *   3. 查询存在的贴图信息
 *   4. 查询不存在的贴图（测试错误处理）
 *   5. 拷贝贴图像素到外部缓冲区验证
 *   6. 发送一条正向滚动弹幕
 *   7. 设置特效（带描边效果 JSON）
 *   8. 渲染一帧到 RGBA 缓冲区
 *   9. 打印渲染结果摘要信息
 *  10. 清理并销毁引擎
 * =============================================================================
 */

#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

/* =============================================================================
 * FFI 函数声明（对应 Rust cdylib 导出的 C 接口）
 *
 * 这些声明需要与 cbindgen 生成的 rs_barrage.h 保持一致。
 * 实际项目中应 #include "rs_barrage.h"。
 * ============================================================================= */

/** 创建弹幕引擎实例，返回不透明指针 */
void* rs_barrage_create(void);

/** 销毁弹幕引擎实例 */
void rs_barrage_destroy(void* ptr);

/**
 * 注册 RGBA 贴图到引擎
 * @param ptr        引擎指针
 * @param id         表情名称（如 "666", "laugh"）
 * @param rgba_data  RGBA8888 像素数据
 * @param width      贴图宽度
 * @param height     贴图高度
 * @return 0=成功, 非0=错误码
 */
int32_t rs_barrage_register_emoji_rgba(void* ptr, const char* id,
                                       const uint8_t* rgba_data,
                                       uint32_t width, uint32_t height);

/**
 * 查询已注册贴图的信息
 * @param ptr         引擎指针
 * @param id          表情名称
 * @param out_width   输出宽度
 * @param out_height  输出高度
 * @param out_bytes_len 输出像素数据总字节数
 * @return 0=存在, 非0=不存在或错误
 */
int32_t rs_barrage_get_emoji_info(void* ptr, const char* id,
                                   uint32_t* out_width,
                                   uint32_t* out_height,
                                   uint64_t* out_bytes_len);

/**
 * 拷贝贴图的像素数据到外部缓冲区
 * @param ptr        引擎指针
 * @param id         表情名称
 * @param out_buffer 输出缓冲区
 * @param buffer_len 缓冲区大小
 * @return 0=成功, 非0=错误
 */
int32_t rs_barrage_copy_emoji_bitmap(void* ptr, const char* id,
                                      uint8_t* out_buffer,
                                      uint64_t buffer_len);

/**
 * 发送一条弹幕
 * @param ptr         引擎指针
 * @param text        弹幕文本
 * @param track_type  轨道类型: 0=正向滚动, 1=顶部固定, 2=底部固定, 3=逆向滚动
 * @param color_rgba  颜色 RGBA 值
 * @param font_size   字体大小
 * @param alpha       透明度 (0.0~1.0)
 * @param duration_ms 持续时间（毫秒）
 * @param effect_json 特效 JSON 配置（可为 NULL）
 * @return 弹幕ID（>0=成功）, <=0=失败
 */
int32_t rs_barrage_send(void* ptr, const char* text, int32_t track_type,
                         uint32_t color_rgba, float font_size, float alpha,
                         uint32_t duration_ms, const char* effect_json);

/**
 * 渲染一帧到 RGBABuffer
 * @param ptr    引擎指针
 * @param buffer 输出 RGBA8888 像素缓冲区
 * @param width  渲染宽度
 * @param height 渲染高度
 * @return 0=成功, 非0=错误
 */
int32_t rs_barrage_render(void* ptr, uint8_t* buffer,
                           uint32_t width, uint32_t height);

/** 清除所有弹幕 */
void rs_barrage_clear(void* ptr);

/** 设置弹幕速度倍率 (0.25~4.0) */
void rs_barrage_set_speed(void* ptr, float speed);

/** 暂停弹幕播放 */
void rs_barrage_pause(void* ptr);

/** 恢复弹幕播放 */
void rs_barrage_resume(void* ptr);

/** 获取引擎版本号，返回静态 C 字符串 */
const char* rs_barrage_version(void);


/* =============================================================================
 * 辅助函数
 * ============================================================================= */

/** 生成 16x16 的纯色 RGBA 测试贴图（上半红色、下半蓝色） */
static uint8_t* create_test_texture(uint32_t width, uint32_t height) {
    uint64_t total = (uint64_t)width * height * 4;
    uint8_t* data = (uint8_t*)malloc(total);
    if (!data) return NULL;

    uint32_t half_h = height / 2;
    for (uint32_t y = 0; y < height; y++) {
        for (uint32_t x = 0; x < width; x++) {
            uint64_t idx = ((uint64_t)y * width + x) * 4;
            if (y < half_h) {
                // 上半部分：红色 (R=255, G=0, B=0, A=255)
                data[idx + 0] = 255;
                data[idx + 1] = 0;
                data[idx + 2] = 0;
                data[idx + 3] = 255;
            } else {
                // 下半部分：蓝色 (R=0, G=0, B=255, A=255)
                data[idx + 0] = 0;
                data[idx + 1] = 0;
                data[idx + 2] = 255;
                data[idx + 3] = 255;
            }
        }
    }
    return data;
}

/** 打印 RGBA 缓冲区的颜色摘要（边界像素采样） */
static void print_buffer_summary(const uint8_t* buffer,
                                  uint32_t width, uint32_t height) {
    if (!buffer || width == 0 || height == 0) {
        printf("  [缓冲区为空或尺寸无效]\n");
        return;
    }

    uint64_t total_bytes = (uint64_t)width * height * 4;
    printf("  缓冲区尺寸: %u x %u, 总字节数: %lu\n",
           width, height, (unsigned long)total_bytes);

    // 采样四个角 + 中心
    uint32_t corners[][2] = {
        {0, 0},                    // 左上
        {width - 1, 0},            // 右上
        {0, height - 1},           // 左下
        {width - 1, height - 1},   // 右下
        {width / 2, height / 2}    // 中心
    };
    const char* corner_names[] = {"左上", "右上", "左下", "右下", "中心"};

    for (int i = 0; i < 5; i++) {
        uint32_t px = corners[i][0];
        uint32_t py = corners[i][1];
        uint64_t idx = ((uint64_t)py * width + px) * 4;
        printf("  [%s] R=%3u G=%3u B=%3u A=%3u\n",
               corner_names[i],
               buffer[idx + 0], buffer[idx + 1],
               buffer[idx + 2], buffer[idx + 3]);
    }
}


/* =============================================================================
 * 主函数 — 完整功能验证流程
 * ============================================================================= */

int main() {
    int ret = 0;
    printf("============================================\n");
    printf("  rs_ffi_barrage — C FFI Demo\n");
    printf("============================================\n\n");

    /* ── 第 0 步：获取版本号 ──────────────────────────────────────────────── */
    const char* version = rs_barrage_version();
    printf("[0] 引擎版本: %s\n\n", version ? version : "(null)");

    /* ── 第 1 步：创建引擎 ───────────────────────────────────────────────── */
    void* engine = rs_barrage_create();
    if (!engine) {
        fprintf(stderr, "[错误] 创建引擎失败！\n");
        return 1;
    }
    printf("[1] 引擎创建成功\n\n");

    /* ── 第 2 步：注册测试贴图 ───────────────────────────────────────────── */
    uint32_t tex_w = 16, tex_h = 16;
    uint8_t* tex_data = create_test_texture(tex_w, tex_h);
    if (!tex_data) {
        fprintf(stderr, "[错误] 无法分配测试贴图内存！\n");
        rs_barrage_destroy(engine);
        return 1;
    }

    const char* emoji_id = "test_emoji";
    ret = rs_barrage_register_emoji_rgba(engine, emoji_id,
                                          tex_data, tex_w, tex_h);
    printf("[2] 注册贴图 [%s] %ux%u: %s\n\n",
           emoji_id, tex_w, tex_h,
           ret == 0 ? "成功" : "失败");

    /* ── 第 3 步：查询存在的贴图信息 ─────────────────────────────────────── */
    uint32_t out_w = 0, out_h = 0;
    uint64_t out_bytes = 0;
    ret = rs_barrage_get_emoji_info(engine, emoji_id,
                                     &out_w, &out_h, &out_bytes);
    if (ret == 0) {
        printf("[3] 查询贴图 [%s]: 宽=%u, 高=%u, 字节数=%lu\n\n",
               emoji_id, out_w, out_h, (unsigned long)out_bytes);
    } else {
        printf("[3] 查询贴图 [%s] 失败 (ret=%d)\n\n", emoji_id, ret);
    }

    /* ── 第 4 步：查询不存在的贴图（测试错误处理） ───────────────────────── */
    const char* fake_id = "non_existent_emoji";
    uint32_t fake_w = 0, fake_h = 0;
    uint64_t fake_bytes = 0;
    ret = rs_barrage_get_emoji_info(engine, fake_id,
                                     &fake_w, &fake_h, &fake_bytes);
    printf("[4] 查询不存在的贴图 [%s]: %s (ret=%d)\n\n",
           fake_id,
           ret != 0 ? "正确返回错误" : "异常返回成功（有误）",
           ret);

    /* ── 第 5 步：拷贝贴图像素到外部缓冲区验证 ──────────────────────────── */
    uint64_t bitmap_size = (uint64_t)out_w * out_h * 4;
    if (bitmap_size > 0) {
        uint8_t* bitmap_buf = (uint8_t*)malloc(bitmap_size);
        if (bitmap_buf) {
            memset(bitmap_buf, 0, bitmap_size);
            ret = rs_barrage_copy_emoji_bitmap(engine, emoji_id,
                                                bitmap_buf, bitmap_size);
            if (ret == 0) {
                printf("[5] 拷贝贴图像素成功，检查像素值:\n");
                // 验证第一个像素是否为红色（上半部分）
                if (bitmap_buf[0] == 255 && bitmap_buf[1] == 0 &&
                    bitmap_buf[2] == 0 && bitmap_buf[3] == 255) {
                    printf("  [验证] 左上角像素 = RGBA(255,0,0,255) — 正确\n");
                }
                // 验证下半部分第一个像素是否为蓝色
                uint64_t half_offset = (uint64_t)(out_h / 2) * out_w * 4;
                if (bitmap_buf[half_offset + 0] == 0 &&
                    bitmap_buf[half_offset + 1] == 0 &&
                    bitmap_buf[half_offset + 2] == 255 &&
                    bitmap_buf[half_offset + 3] == 255) {
                    printf("  [验证] 下半部分首个像素 = RGBA(0,0,255,255) — 正确\n");
                }
            } else {
                printf("[5] 拷贝贴图像素失败 (ret=%d)\n", ret);
            }
            free(bitmap_buf);
        }
        printf("\n");
    }

    /* ── 第 6 步：发送一条弹幕 ───────────────────────────────────────────── */
    int32_t barrage_id = rs_barrage_send(engine,
                                          "Hello, rs_ffi_barrage!",  // 弹幕文本
                                          0,                          // 轨道类型: 正向滚动
                                          0xFFFFFFFF,                 // 颜色: 白色
                                          24.0f,                      // 字号
                                          0.9f,                       // 透明度
                                          5000,                       // 持续 5 秒
                                          NULL);                      // 无特效
    printf("[6] 发送弹幕: ID=%d (%s)\n\n",
           barrage_id,
           barrage_id > 0 ? "成功" : "失败");

    /* ── 第 7 步：发送一条带特效的弹幕 ───────────────────────────────────── */
    // 描边特效 JSON: 红色描边，宽度 2px
    const char* effect_json = "{"
        "\"type\":\"stroke\","
        "\"color\":\"#FF0000\","
        "\"width\":2.0"
    "}";
    int32_t fx_barrage_id = rs_barrage_send(engine,
                                              "特效弹幕测试!",
                                              0,            // 正向滚动
                                              0xFFFF00FF,   // 黄色
                                              28.0f,
                                              1.0f,
                                              6000,
                                              effect_json);
    printf("[7] 发送特效弹幕: ID=%d (%s)\n\n",
           fx_barrage_id,
           fx_barrage_id > 0 ? "成功" : "失败");

    /* ── 第 8 步：渲染一帧 ───────────────────────────────────────────────── */
    uint32_t fb_w = 640, fb_h = 480;
    uint64_t fb_size = (uint64_t)fb_w * fb_h * 4;
    uint8_t* framebuffer = (uint8_t*)malloc(fb_size);
    if (!framebuffer) {
        fprintf(stderr, "[错误] 无法分配帧缓冲区！\n");
        free(tex_data);
        rs_barrage_destroy(engine);
        return 1;
    }
    memset(framebuffer, 0, fb_size);

    ret = rs_barrage_render(engine, framebuffer, fb_w, fb_h);
    printf("[8] 渲染帧 %ux%u: %s\n\n",
           fb_w, fb_h,
           ret == 0 ? "成功" : "失败");

    /* ── 第 9 步：打印渲染结果摘要 ───────────────────────────────────────── */
    printf("[9] 渲染结果摘要:\n");
    print_buffer_summary(framebuffer, fb_w, fb_h);
    printf("\n");

    /* ── 第 10 步：测试速度控制 ──────────────────────────────────────────── */
    printf("[10] 设置速度 2.0x...\n");
    rs_barrage_set_speed(engine, 2.0f);
    printf("  速度已设置为 2.0x\n\n");

    /* ── 第 11 步：暂停/恢复测试 ─────────────────────────────────────────── */
    printf("[11] 暂停弹幕...\n");
    rs_barrage_pause(engine);
    printf("  弹幕已暂停\n");
    printf("  恢复弹幕...\n");
    rs_barrage_resume(engine);
    printf("  弹幕已恢复\n\n");

    /* ── 第 12 步：清除所有弹幕 ──────────────────────────────────────────── */
    printf("[12] 清除所有弹幕...\n");
    rs_barrage_clear(engine);
    printf("  所有弹幕已清除\n\n");

    /* ── 第 13 步：清理资源 ───────────────────────────────────────────────── */
    free(tex_data);
    free(framebuffer);
    printf("[13] 临时内存已释放\n\n");

    /* ── 第 14 步：销毁引擎 ──────────────────────────────────────────────── */
    rs_barrage_destroy(engine);
    printf("[14] 引擎已销毁\n");

    /* ── 完成 ─────────────────────────────────────────────────────────────── */
    printf("\n============================================\n");
    printf("  C FFI Demo 全部执行完成！\n");
    printf("============================================\n");

    return 0;
}