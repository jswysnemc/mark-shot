#include "media.h"
#ifndef _WIN32
#include <pulse/pulseaudio.h>
#include <dlfcn.h>
#include <stdlib.h>
#include <string.h>

#define PULSE_SYMBOLS(X) \
 X(pa_mainloop_new) X(pa_mainloop_free) X(pa_mainloop_get_api) X(pa_mainloop_iterate) \
 X(pa_context_new) X(pa_context_connect) X(pa_context_get_state) X(pa_context_disconnect) X(pa_context_unref) \
 X(pa_context_get_server_info) X(pa_context_get_sink_info_by_name) X(pa_operation_unref) \
 X(pa_stream_new) X(pa_stream_unref) X(pa_stream_disconnect) X(pa_stream_get_state) \
 X(pa_stream_connect_record) X(pa_stream_peek) X(pa_stream_drop) X(pa_stream_get_latency) X(pa_stream_readable_size) \
 X(pa_stream_cork) X(pa_stream_flush)
#define FIELD(name) __typeof__(&name) name;
typedef struct {
    void *library;
    PULSE_SYMBOLS(FIELD)
    pa_mainloop *loop;
    pa_context *context;
    pa_stream *stream;
    int microphone, requested, failed, paused;
} ms_pulse;
#undef FIELD

/** 【原生音频】【流连接】按 source 在 capture 创建 48kHz 浮点双声道输入，队列限制为约两百毫秒 */
static void connect_source(ms_pulse *capture, const char *source) {
    pa_sample_spec specification = { PA_SAMPLE_FLOAT32LE, 48000, 2 };
    capture->stream = capture->pa_stream_new(capture->context, "Mark Shot recording", &specification, NULL);
    pa_buffer_attr buffers = { .maxlength = 48000 * 8 / 5, .tlength = (uint32_t)-1,
        .prebuf = (uint32_t)-1, .minreq = (uint32_t)-1, .fragsize = 480 * 8 };
    pa_stream_flags_t flags = PA_STREAM_ADJUST_LATENCY | PA_STREAM_AUTO_TIMING_UPDATE | PA_STREAM_INTERPOLATE_TIMING;
    if (capture->paused) flags |= PA_STREAM_START_CORKED;
    if (!capture->stream || capture->pa_stream_connect_record(capture->stream, source, &buffers, flags) < 0)
        capture->failed = 1;
}
/** 【原生音频】【输出监听】按 info 的 monitor_source_name 捕获系统输出，不把扬声器误作麦克风 */
static void sink_info(pa_context *context, const pa_sink_info *info, int end, void *data) {
    (void)context;
    ms_pulse *capture = data;
    if (info) connect_source(capture, info->monitor_source_name);
    else if (end < 0) capture->failed = 1;
}
/** 【原生音频】【默认来源】按 info 选择默认麦克风或默认输出的监听流，data 持有请求生命周期 */
static void server_info(pa_context *context, const pa_server_info *info, void *data) {
    ms_pulse *capture = data;
    if (!info) { capture->failed = 1; return; }
    if (capture->microphone) connect_source(capture, info->default_source_name);
    else {
        pa_operation *operation = capture->pa_context_get_sink_info_by_name(context, info->default_sink_name, sink_info, data);
        if (operation) capture->pa_operation_unref(operation);
        else capture->failed = 1;
    }
}
/** 【原生音频】【资源回收】按 handle 先销毁连接与事件循环，最后卸载可选系统库 */
void ms_pulse_destroy(void *handle) {
    ms_pulse *capture = handle;
    if (!capture) return;
    if (capture->stream) { capture->pa_stream_disconnect(capture->stream); capture->pa_stream_unref(capture->stream); }
    if (capture->context) { capture->pa_context_disconnect(capture->context); capture->pa_context_unref(capture->context); }
    if (capture->loop) capture->pa_mainloop_free(capture->loop);
    if (capture->library) dlclose(capture->library);
    free(capture);
}
/** 【原生音频】【可选连接】按 microphone 创建非阻塞 PulseAudio 会话，缺库或连接失败返回空值 */
void *ms_pulse_open(int microphone) {
    ms_pulse *capture = calloc(1, sizeof(*capture));
    if (!capture) return NULL;
    capture->library = dlopen("libpulse.so.0", RTLD_NOW | RTLD_LOCAL);
    if (!capture->library) goto failed;
#define LOAD(name) capture->name = dlsym(capture->library, #name); if (!capture->name) goto failed;
    PULSE_SYMBOLS(LOAD)
#undef LOAD
    capture->microphone = microphone;
    capture->loop = capture->pa_mainloop_new();
    if (!capture->loop) goto failed;
    capture->context = capture->pa_context_new(capture->pa_mainloop_get_api(capture->loop), "Mark Shot");
    if (!capture->context || capture->pa_context_connect(capture->context, NULL, PA_CONTEXT_NOAUTOSPAWN, NULL) < 0) goto failed;
    return capture;
failed:
    ms_pulse_destroy(capture);
    return NULL;
}
/** 【原生音频】【有界读取】按 capacity 交付最多一个音频块，latency 返回首样本距当前时刻的微秒，负数表示失败 */
int ms_pulse_read(void *handle, float *samples, int capacity, uint64_t *latency) {
    ms_pulse *capture = handle;
    for (int i = 0; i < 32; ++i) {
        int result = capture->pa_mainloop_iterate(capture->loop, 0, NULL);
        if (result < 0) return -1;
        if (!result) break;
    }
    pa_context_state_t state = capture->pa_context_get_state(capture->context);
    if (state == PA_CONTEXT_FAILED || state == PA_CONTEXT_TERMINATED || capture->failed) return -1;
    if (state == PA_CONTEXT_READY && !capture->requested) {
        capture->requested = 1;
        pa_operation *operation = capture->pa_context_get_server_info(capture->context, server_info, capture);
        if (!operation) return -1;
        capture->pa_operation_unref(operation);
    }
    if (!capture->stream) return 0;
    pa_stream_state_t stream_state = capture->pa_stream_get_state(capture->stream);
    if (stream_state == PA_STREAM_FAILED || stream_state == PA_STREAM_TERMINATED) return -1;
    if (stream_state != PA_STREAM_READY) return 0;
    size_t available = capture->pa_stream_readable_size(capture->stream);
    if (available == (size_t)-1) return -1;
    if (!available) return 0;
    const void *buffer; size_t bytes;
    if (capture->pa_stream_peek(capture->stream, &buffer, &bytes) < 0 || bytes > (size_t)capacity * 8 || bytes % 8) return -1;
    int negative = 0;
    if (capture->pa_stream_get_latency(capture->stream, latency, &negative) < 0 || negative) *latency = bytes * 1000000 / (48000 * 8);
    if (buffer) memcpy(samples, buffer, bytes); else memset(samples, 0, bytes);
    if (bytes && capture->pa_stream_drop(capture->stream) < 0) return -1;
    return bytes / 8;
}
/** 【原生音频】【暂停控制】按 paused 顺序暂停、清空与恢复服务器队列，后续 read 推进确认事件 */
int ms_pulse_pause(void *handle, int paused) {
    ms_pulse *capture = handle;
    capture->paused = paused;
    if (!capture->stream || capture->pa_stream_get_state(capture->stream) != PA_STREAM_READY) return 0;
    pa_operation *operation = capture->pa_stream_cork(capture->stream, 1, NULL, NULL);
    if (!operation) return -1;
    capture->pa_operation_unref(operation);
    operation = capture->pa_stream_flush(capture->stream, NULL, NULL);
    if (!operation) return -1;
    capture->pa_operation_unref(operation);
    if (!paused) {
        operation = capture->pa_stream_cork(capture->stream, 0, NULL, NULL);
        if (!operation) return -1;
        capture->pa_operation_unref(operation);
    }
    return 0;
}
#endif
