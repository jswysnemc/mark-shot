#ifndef MARK_SHOT_MEDIA_H
#define MARK_SHOT_MEDIA_H

#include <stdint.h>
#include <stddef.h>
#ifdef _WIN32
#define MS_API __declspec(dllexport)
#else
#define MS_API __attribute__((visibility("default")))
#endif

typedef struct ms_encoder ms_encoder;
typedef int (*ms_cancel)(void *context);
typedef struct {
    int width, height;
    uint32_t format;
    int planes;
    uint64_t modifier;
    int fd[4];
    uint32_t size[4], offset[4], stride[4];
} ms_dmabuf;
MS_API void *ms_surface_open(int device, int width, int height, uint32_t format,
    const uint64_t *modifiers, int count, ms_dmabuf *descriptor);
MS_API void ms_surface_destroy(void *handle);
MS_API int ms_encoder_dmabuf(ms_encoder *encoder, const ms_dmabuf *descriptor, int x, int y, int64_t pts_us);
#ifndef _WIN32
MS_API void *ms_pulse_open(int microphone);
MS_API void ms_pulse_destroy(void *handle);
MS_API int ms_pulse_read(void *handle, float *samples, int capacity, uint64_t *latency);
MS_API int ms_pulse_pause(void *handle, int paused);
#endif

MS_API uint32_t ms_media_abi(void);
MS_API ms_encoder *ms_encoder_open(const char *path, const char *device, int width, int height, int fps,
    int quality, int mode, int audio, char *error, size_t capacity);
MS_API const char *ms_encoder_error(ms_encoder *encoder);
MS_API const char *ms_encoder_name(ms_encoder *encoder);
MS_API int ms_encoder_video(ms_encoder *encoder, const uint8_t *rgba, int stride, int64_t pts_us);
MS_API int ms_encoder_audio(ms_encoder *encoder, const float *samples, int frames, int64_t pts_us);
MS_API int ms_encoder_finish(ms_encoder *encoder);
MS_API void ms_encoder_destroy(ms_encoder *encoder);
MS_API int ms_export(const char *source, const char *destination, int gif_fps,
    ms_cancel cancelled, void *context, char *error, size_t capacity);
#endif
