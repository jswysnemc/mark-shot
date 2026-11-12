#include "internal.h"
#ifndef _WIN32
#include <gbm.h>
#include <unistd.h>
#include <sys/stat.h>
#include <libavutil/hwcontext_vaapi.h>
#include <va/va_drmcommon.h>
#include <va/va_vpp.h>

typedef struct { int fd; struct gbm_device *device; struct gbm_bo *buffer; } ms_surface;

/** 【显存帧】【资源释放】按 handle 先释放缓冲与设备，再关闭本对象拥有的设备描述符 */
void ms_surface_destroy(void *handle) {
    ms_surface *surface = handle;
    if (!surface) return;
    if (surface->buffer) gbm_bo_destroy(surface->buffer);
    if (surface->device) gbm_device_destroy(surface->device);
    if (surface->fd >= 0) close(surface->fd);
    av_free(surface);
}

/** 【显存帧】【格式约束】按 format 返回对应 VA 四字符格式，不接受未实现的多层颜色布局 */
static uint32_t va_format(uint32_t format) {
    switch (format) {
    case GBM_FORMAT_XRGB8888: return VA_FOURCC_BGRX;
    case GBM_FORMAT_ARGB8888: return VA_FOURCC_BGRA;
    case GBM_FORMAT_XBGR8888: return VA_FOURCC_RGBX;
    case GBM_FORMAT_ABGR8888: return VA_FOURCC_RGBA;
    default: return 0;
    }
}

/** 【显存帧】【兼容分配】按设备、尺寸、格式和允许的 modifiers 建立单个 GBM 缓冲，descriptor 的 FD 由调用者关闭 */
void *ms_surface_open(int device, int width, int height, uint32_t format,
    const uint64_t *modifiers, int count, ms_dmabuf *descriptor) {
    if (!descriptor || width < 2 || height < 2 || width > 16384 || height > 16384 ||
        (int64_t)width * height > 33554432 || !va_format(format) || count < 1 || count > 256) return NULL;
    ms_surface *surface = av_mallocz(sizeof(*surface));
    if (!surface) return NULL;
    surface->fd = dup(device);
    if (surface->fd < 0) goto failed;
    surface->device = gbm_create_device(surface->fd);
    if (!surface->device) goto failed;
    surface->buffer = gbm_bo_create_with_modifiers2(surface->device, width, height, format,
        modifiers, count, GBM_BO_USE_RENDERING);
    if (!surface->buffer) goto failed;
    int planes = gbm_bo_get_plane_count(surface->buffer);
    if (planes < 1 || planes > 4) goto failed;
    memset(descriptor, 0, sizeof(*descriptor));
    descriptor->width = width; descriptor->height = height; descriptor->format = format;
    descriptor->planes = planes; descriptor->modifier = gbm_bo_get_modifier(surface->buffer);
    for (int i = 0; i < planes; ++i) {
        descriptor->fd[i] = gbm_bo_get_fd_for_plane(surface->buffer, i);
        if (descriptor->fd[i] < 0) {
            for (int j = 0; j < i; ++j) close(descriptor->fd[j]);
            goto failed;
        }
        off_t length = lseek(descriptor->fd[i], 0, SEEK_END);
        lseek(descriptor->fd[i], 0, SEEK_SET);
        if (length <= 0 || (uint64_t)length > UINT32_MAX) {
            for (int j = 0; j <= i; ++j) close(descriptor->fd[j]);
            goto failed;
        }
        descriptor->size[i] = length;
        descriptor->offset[i] = gbm_bo_get_offset(surface->buffer, i);
        descriptor->stride[i] = gbm_bo_get_stride_for_plane(surface->buffer, i);
    }
    return surface;
failed:
    ms_surface_destroy(surface);
    return NULL;
}

/** 【显存帧】【同步导入】将 descriptor 指定的已完成帧裁剪到编码器私有 NV12，返回前等待显存读取完成 */
int ms_encoder_dmabuf(ms_encoder *encoder, const ms_dmabuf *descriptor, int x, int y, int64_t pts_us) {
    if (!encoder->device || encoder->finalized || pts_us <= encoder->last_video ||
        descriptor->planes < 1 || descriptor->planes > 4 || !va_format(descriptor->format) ||
        descriptor->width > 16384 || descriptor->height > 16384 || x < 0 || y < 0 ||
        x + encoder->video->width > descriptor->width || y + encoder->video->height > descriptor->height)
        return ms_fail(encoder, "incompatible DMA-BUF frame", AVERROR(EINVAL));
    AVHWDeviceContext *device = (AVHWDeviceContext *)encoder->device->data;
    if (device->type != AV_HWDEVICE_TYPE_VAAPI) return ms_fail(encoder, "DMA-BUF requires VAAPI", AVERROR(ENOSYS));
    VADisplay display = ((AVVAAPIDeviceContext *)device->hwctx)->display;
    VADRMPRIMESurfaceDescriptor input = {0};
    input.fourcc = va_format(descriptor->format); input.width = descriptor->width; input.height = descriptor->height;
    input.num_objects = descriptor->planes; input.num_layers = 1;
    input.layers[0].drm_format = descriptor->format; input.layers[0].num_planes = descriptor->planes;
    for (int i = 0; i < descriptor->planes; ++i) {
        if (descriptor->fd[i] < 0 || !descriptor->size[i] || descriptor->offset[i] >= descriptor->size[i] || !descriptor->stride[i])
            return ms_fail(encoder, "invalid DMA-BUF plane", AVERROR(EINVAL));
        input.objects[i].fd = descriptor->fd[i]; input.objects[i].size = descriptor->size[i];
        input.objects[i].drm_format_modifier = descriptor->modifier;
        input.layers[0].object_index[i] = i;
        input.layers[0].offset[i] = descriptor->offset[i]; input.layers[0].pitch[i] = descriptor->stride[i];
    }
    VASurfaceAttrib attributes[2] = {0};
    attributes[0].type = VASurfaceAttribMemoryType; attributes[0].flags = VA_SURFACE_ATTRIB_SETTABLE;
    attributes[0].value.type = VAGenericValueTypeInteger; attributes[0].value.value.i = VA_SURFACE_ATTRIB_MEM_TYPE_DRM_PRIME_2;
    attributes[1].type = VASurfaceAttribExternalBufferDescriptor; attributes[1].flags = VA_SURFACE_ATTRIB_SETTABLE;
    attributes[1].value.type = VAGenericValueTypePointer; attributes[1].value.value.p = &input;
    VASurfaceID source = VA_INVALID_SURFACE, destination = VA_INVALID_SURFACE;
    VAConfigID config = VA_INVALID_ID;
    VAContextID context = VA_INVALID_ID;
    VABufferID parameters = VA_INVALID_ID;
    VAStatus status = vaCreateSurfaces(display, VA_RT_FORMAT_RGB32, input.width, input.height, &source, 1, attributes, 2);
    int result = AVERROR(EIO);
    if (status != VA_STATUS_SUCCESS) goto end;
    av_frame_unref(encoder->hardware);
    result = av_hwframe_get_buffer(encoder->video->hw_frames_ctx, encoder->hardware, 0);
    if (result < 0) goto end;
    destination = (VASurfaceID)(uintptr_t)encoder->hardware->data[3];
    status = vaCreateConfig(display, VAProfileNone, VAEntrypointVideoProc, NULL, 0, &config);
    if (status != VA_STATUS_SUCCESS) goto end;
    status = vaCreateContext(display, config, encoder->video->width, encoder->video->height, VA_PROGRESSIVE, &destination, 1, &context);
    if (status != VA_STATUS_SUCCESS) goto end;
    VARectangle crop = {x, y, encoder->video->width, encoder->video->height};
    VAProcPipelineParameterBuffer pipeline = {0};
    pipeline.surface = source; pipeline.surface_region = &crop;
    pipeline.surface_color_standard = VAProcColorStandardBT709;
    pipeline.output_color_standard = VAProcColorStandardBT709;
    pipeline.input_color_properties.color_range = VA_SOURCE_RANGE_FULL;
    pipeline.output_color_properties.color_range = VA_SOURCE_RANGE_REDUCED;
    status = vaCreateBuffer(display, context, VAProcPipelineParameterBufferType, sizeof(pipeline), 1, &pipeline, &parameters);
    if (status != VA_STATUS_SUCCESS) goto end;
    status = vaBeginPicture(display, context, destination);
    if (status != VA_STATUS_SUCCESS) goto end;
    status = vaRenderPicture(display, context, &parameters, 1);
    VAStatus ended = vaEndPicture(display, context);
    // 1. 【显存帧】【生产者租约】即使转换失败也等待已提交工作，调用返回后生产者才能复用输入
    VAStatus synced = vaSyncSurface(display, destination);
    if (status == VA_STATUS_SUCCESS) status = ended;
    if (status == VA_STATUS_SUCCESS) status = synced;
    if (status != VA_STATUS_SUCCESS) goto end;
    encoder->hardware->pts = pts_us;
    encoder->hardware->duration = av_rescale_q(1, av_inv_q(encoder->video->framerate), AV_TIME_BASE_Q);
    result = ms_write(encoder, encoder->video, encoder->video_stream, encoder->hardware);
    if (result >= 0) encoder->last_video = pts_us;
end:
    if (parameters != VA_INVALID_ID) vaDestroyBuffer(display, parameters);
    if (context != VA_INVALID_ID) vaDestroyContext(display, context);
    if (config != VA_INVALID_ID) vaDestroyConfig(display, config);
    if (source != VA_INVALID_SURFACE) vaDestroySurfaces(display, &source, 1);
    if (status != VA_STATUS_SUCCESS) {
        snprintf(encoder->error, sizeof(encoder->error), "DMA-BUF VAAPI: %s", vaErrorStr(status));
        encoder->failed = 1;
        return AVERROR(EIO);
    }
    return result;
}
#else
/** 【显存帧】【平台边界】Windows 由 D3D11 原生录制，此私有导出库不接受 Linux 句柄 */
void *ms_surface_open(int device, int width, int height, uint32_t format, const uint64_t *modifiers, int count, ms_dmabuf *descriptor) {
    (void)device; (void)width; (void)height; (void)format; (void)modifiers; (void)count; (void)descriptor; return NULL;
}
/** 【显存帧】【平台回收】Windows 无 Linux surface 可回收 */
void ms_surface_destroy(void *handle) { (void)handle; }
/** 【显存帧】【平台拒绝】Windows 不接受 DMA-BUF，返回不支持 */
int ms_encoder_dmabuf(ms_encoder *encoder, const ms_dmabuf *descriptor, int x, int y, int64_t pts_us) {
    (void)descriptor; (void)x; (void)y; (void)pts_us; return ms_fail(encoder, "DMA-BUF unavailable", AVERROR(ENOSYS));
}
#endif
