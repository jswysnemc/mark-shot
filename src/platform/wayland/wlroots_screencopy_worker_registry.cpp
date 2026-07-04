#include "platform/wayland/wlroots_screencopy_worker.h"

#include <QPoint>
#include <QSize>

#include <algorithm>
#include <cstring>
#include <utility>

namespace markshot::recording {

#ifdef HAVE_WLROOTS_SCREENCOPY

/**
 * 【录制】【wlroots采集】处理 Wayland registry 全局对象。
 * @param data 工作对象指针。
 * @param registry registry 对象。
 * @param name 全局对象名称。
 * @param interface 接口名称。
 * @param version 接口版本。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleRegistryGlobal(void *data,
                                                   wl_registry *registry,
                                                   uint32_t name,
                                                   const char *interface,
                                                   uint32_t version)
{
    auto *self = static_cast<WlrootsScreencopyWorker *>(data);
    if (!self || !interface) {
        return;
    }
    if (std::strcmp(interface, wl_shm_interface.name) == 0) {
        self->m_shm = static_cast<wl_shm *>(wl_registry_bind(registry, name, &wl_shm_interface, 1));
    } else if (std::strcmp(interface, wl_output_interface.name) == 0) {
        auto output = std::make_unique<WlrootsOutput>();
        const uint32_t bindVersion = std::min<uint32_t>(version, 4);
        output->output = static_cast<wl_output *>(
            wl_registry_bind(registry, name, &wl_output_interface, bindVersion));
        wl_output_add_listener(output->output, &s_outputListener, output.get());
        self->m_outputs.push_back(std::move(output));
    } else if (std::strcmp(interface, zwlr_screencopy_manager_v1_interface.name) == 0) {
        self->m_screencopyVersion = std::min<uint32_t>(version, 3);
        self->m_screencopy = static_cast<zwlr_screencopy_manager_v1 *>(
            wl_registry_bind(registry,
                             name,
                             &zwlr_screencopy_manager_v1_interface,
                             self->m_screencopyVersion));
    }
}

/**
 * 【录制】【wlroots采集】处理 Wayland registry 移除事件。
 * @param data 工作对象指针。
 * @param registry registry 对象。
 * @param name 全局对象名称。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleRegistryRemove(void *data,
                                                   wl_registry *registry,
                                                   uint32_t name)
{
    Q_UNUSED(data)
    Q_UNUSED(registry)
    Q_UNUSED(name)
}

/**
 * 【录制】【wlroots采集】记录输出几何信息。
 * @param data 输出状态。
 * @param output Wayland 输出。
 * @param x 输出横坐标。
 * @param y 输出纵坐标。
 * @param physicalWidth 物理宽度。
 * @param physicalHeight 物理高度。
 * @param subpixel 子像素布局。
 * @param make 制造商。
 * @param model 型号。
 * @param transform 输出变换。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleOutputGeometry(void *data,
                                                   wl_output *output,
                                                   int32_t x,
                                                   int32_t y,
                                                   int32_t physicalWidth,
                                                   int32_t physicalHeight,
                                                   int32_t subpixel,
                                                   const char *make,
                                                   const char *model,
                                                   int32_t transform)
{
    Q_UNUSED(output)
    Q_UNUSED(physicalWidth)
    Q_UNUSED(physicalHeight)
    Q_UNUSED(subpixel)
    Q_UNUSED(make)
    Q_UNUSED(model)
    Q_UNUSED(transform)
    auto *state = static_cast<WlrootsOutput *>(data);
    if (state) {
        state->geometry.moveTopLeft(QPoint(x, y));
    }
}

/**
 * 【录制】【wlroots采集】记录输出模式尺寸。
 * @param data 输出状态。
 * @param output Wayland 输出。
 * @param flags 模式标志。
 * @param width 输出宽度。
 * @param height 输出高度。
 * @param refresh 输出刷新率。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleOutputMode(void *data,
                                               wl_output *output,
                                               uint32_t flags,
                                               int32_t width,
                                               int32_t height,
                                               int32_t refresh)
{
    Q_UNUSED(output)
    Q_UNUSED(flags)
    Q_UNUSED(refresh)
    auto *state = static_cast<WlrootsOutput *>(data);
    if (state && width > 0 && height > 0) {
        state->geometry.setSize(QSize(width, height));
    }
}

/**
 * 【录制】【wlroots采集】处理输出完成事件。
 * @param data 输出状态。
 * @param output Wayland 输出。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleOutputDone(void *data, wl_output *output)
{
    Q_UNUSED(data)
    Q_UNUSED(output)
}

/**
 * 【录制】【wlroots采集】记录输出缩放。
 * @param data 输出状态。
 * @param output Wayland 输出。
 * @param scale 输出缩放。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleOutputScale(void *data, wl_output *output, int32_t scale)
{
    Q_UNUSED(output)
    auto *state = static_cast<WlrootsOutput *>(data);
    if (state && scale > 0) {
        state->scale = scale;
    }
}

/**
 * 【录制】【wlroots采集】记录输出名称。
 * @param data 输出状态。
 * @param output Wayland 输出。
 * @param name 输出名称。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleOutputName(void *data, wl_output *output, const char *name)
{
    Q_UNUSED(output)
    auto *state = static_cast<WlrootsOutput *>(data);
    if (state && name) {
        state->name = QString::fromUtf8(name);
    }
}

/**
 * 【录制】【wlroots采集】记录输出描述。
 * @param data 输出状态。
 * @param output Wayland 输出。
 * @param description 输出描述。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleOutputDescription(void *data,
                                                      wl_output *output,
                                                      const char *description)
{
    Q_UNUSED(output)
    auto *state = static_cast<WlrootsOutput *>(data);
    if (state && description) {
        state->description = QString::fromUtf8(description);
    }
}

/**
 * 【录制】【wlroots采集】选择当前录制目标输出。
 * @return 输出状态，找不到时返回空。
 */
WlrootsOutput *WlrootsScreencopyWorker::chooseOutput() const
{
    // 1. 【录制】【wlroots采集】优先匹配配置中明确指定的输出名称
    if (!m_options.display.outputName.isEmpty()) {
        for (const std::unique_ptr<WlrootsOutput> &output : m_outputs) {
            if (output->name == m_options.display.outputName) {
                return output.get();
            }
        }
    }

    // 2. 【录制】【wlroots采集】其次用录制区域中心点推断所在输出
    const QRect requested = m_options.captureGeometry.normalized();
    if (requested.isValid() && !requested.isEmpty()) {
        const QPoint center = requested.center();
        for (const std::unique_ptr<WlrootsOutput> &output : m_outputs) {
            if (output->geometry.contains(center)) {
                return output.get();
            }
        }
    }

    // 3. 【录制】【wlroots采集】单输出环境直接使用唯一输出，多输出未知目标时交给上层回退
    return m_outputs.size() == 1 ? m_outputs.front().get() : nullptr;
}

const wl_registry_listener WlrootsScreencopyWorker::s_registryListener{
    &WlrootsScreencopyWorker::handleRegistryGlobal,
    &WlrootsScreencopyWorker::handleRegistryRemove,
};

const wl_output_listener WlrootsScreencopyWorker::s_outputListener{
    &WlrootsScreencopyWorker::handleOutputGeometry,
    &WlrootsScreencopyWorker::handleOutputMode,
    &WlrootsScreencopyWorker::handleOutputDone,
    &WlrootsScreencopyWorker::handleOutputScale,
    &WlrootsScreencopyWorker::handleOutputName,
    &WlrootsScreencopyWorker::handleOutputDescription,
};

#endif

}  // namespace markshot::recording
