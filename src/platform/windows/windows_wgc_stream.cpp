#include "platform/windows/windows_wgc_stream.h"

#include "debug_log.h"
#include "platform/windows/windows_wgc_support.h"
#include "recording/recording_bgra_buffer_pool.h"

#include <QElapsedTimer>

#include <atomic>
#include <cstring>
#include <limits>
#include <mutex>
#include <optional>
#include <utility>

#if defined(Q_OS_WIN)

namespace markshot::platform::windows {
namespace {

namespace Capture = wgc_support::Capture;
namespace Direct3D11 = wgc_support::Direct3D11;
namespace Direct3D11Abi = wgc_support::Direct3D11Abi;
namespace DirectX = wgc_support::DirectX;

using namespace wgc_support;

struct StreamState {
    D3DDeviceBundle bundle;
    WindowsWgcStream::FrameCallback onFrame;
    WindowsWgcStream::ErrorCallback onError;
    QElapsedTimer elapsed;
    QRect cropPixels;
    QSize outputSize;
    std::mutex frameMutex;
    winrt::com_ptr<ID3D11Texture2D> staging;
    markshot::recording::RecordingBgraBufferPool bgraPool;
    UINT stagingWidth = 0;
    UINT stagingHeight = 0;
    DXGI_FORMAT stagingFormat = DXGI_FORMAT_UNKNOWN;
    std::atomic_bool running{false};
    std::atomic_bool errorReported{false};
    std::atomic_bool backpressure{false};
};

/**
 * 【录制】【Windows采集】通知一次采集错误。
 * @param state 采集状态。
 * @param errorText 错误文本。
 * @return 无返回值。
 */
void reportStreamError(const std::shared_ptr<StreamState> &state, const QString &errorText)
{
    if (!state || errorText.isEmpty() || state->errorReported.exchange(true)) {
        return;
    }
    const auto callback = state->onError;
    if (callback) {
        callback(errorText);
    }
}

/**
 * 【录制】【Windows采集】确保 staging 纹理可复用。
 * @param state 采集状态。
 * @param desc 当前源纹理描述。
 * @param error 输出错误信息。
 * @return 可用时返回 true。
 */
bool ensureStagingTexture(const std::shared_ptr<StreamState> &state,
                          const D3D11_TEXTURE2D_DESC &desc,
                          QString *error)
{
    if (state->staging
        && state->stagingWidth == desc.Width
        && state->stagingHeight == desc.Height
        && state->stagingFormat == desc.Format) {
        return true;
    }

    D3D11_TEXTURE2D_DESC stagingDesc = desc;
    stagingDesc.BindFlags = 0;
    stagingDesc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    stagingDesc.MiscFlags = 0;
    stagingDesc.Usage = D3D11_USAGE_STAGING;

    winrt::com_ptr<ID3D11Texture2D> staging;
    const HRESULT result = state->bundle.device->CreateTexture2D(&stagingDesc, nullptr, staging.put());
    if (FAILED(result)) {
        if (error) {
            *error = hresultError(QStringLiteral("failed to create staging texture"), result);
        }
        return false;
    }

    state->staging = std::move(staging);
    state->stagingWidth = desc.Width;
    state->stagingHeight = desc.Height;
    state->stagingFormat = desc.Format;
    return true;
}

/**
 * 【录制】【Windows采集】把 WGC 帧复制为连续 BGRA 数据。
 * @param state 采集状态。
 * @param frame WGC 帧。
 * @param error 输出错误信息。
 * @return 成功时返回 BGRA 帧。
 */
std::optional<WindowsWgcFrame> readFrameToBgra(const std::shared_ptr<StreamState> &state,
                                               const Capture::Direct3D11CaptureFrame &frame,
                                               QString *error)
{
    const Direct3D11::IDirect3DSurface surface = frame.Surface();
    if (!surface) {
        if (error) {
            *error = QStringLiteral("Windows Graphics Capture returned an empty surface");
        }
        return std::nullopt;
    }

    winrt::com_ptr<Direct3D11Abi::IDirect3DDxgiInterfaceAccess> access =
        surface.as<Direct3D11Abi::IDirect3DDxgiInterfaceAccess>();
    winrt::com_ptr<ID3D11Texture2D> texture;
    HRESULT result = access->GetInterface(__uuidof(ID3D11Texture2D), texture.put_void());
    if (FAILED(result)) {
        if (error) {
            *error = hresultError(QStringLiteral("failed to access capture texture"), result);
        }
        return std::nullopt;
    }

    D3D11_TEXTURE2D_DESC desc = {};
    texture->GetDesc(&desc);
    if (desc.Width == 0 || desc.Height == 0
        || desc.Width > static_cast<UINT>(std::numeric_limits<int>::max())
        || desc.Height > static_cast<UINT>(std::numeric_limits<int>::max())) {
        if (error) {
            *error = QStringLiteral("Windows Graphics Capture returned invalid frame dimensions");
        }
        return std::nullopt;
    }

    std::lock_guard<std::mutex> lock(state->frameMutex);
    if (!ensureStagingTexture(state, desc, error)) {
        return std::nullopt;
    }

    state->bundle.context->CopyResource(state->staging.get(), texture.get());

    D3D11_MAPPED_SUBRESOURCE mapped = {};
    result = state->bundle.context->Map(state->staging.get(), 0, D3D11_MAP_READ, 0, &mapped);
    if (FAILED(result)) {
        if (error) {
            *error = hresultError(QStringLiteral("failed to map staging texture"), result);
        }
        return std::nullopt;
    }

    const QSize frameSize(static_cast<int>(desc.Width), static_cast<int>(desc.Height));
    const QRect sourceCrop = state->cropPixels.intersected(QRect(QPoint(0, 0), frameSize));
    const QRect crop = sourceCrop.isEmpty() ? QRect(QPoint(0, 0), frameSize) : sourceCrop;
    const int rowBytes = crop.width() * 4;
    // 复用池化缓冲承接 staging 纹理数据，避免每帧新分配整帧内存
    QByteArray &bgra = state->bgraPool.acquire(static_cast<qsizetype>(rowBytes) * crop.height());
    for (int y = 0; y < crop.height(); ++y) {
        const auto *source = static_cast<const uchar *>(mapped.pData)
            + mapped.RowPitch * (crop.y() + y)
            + crop.x() * 4;
        std::memcpy(bgra.data() + static_cast<qsizetype>(rowBytes) * y,
                    source,
                    static_cast<std::size_t>(rowBytes));
    }

    state->bundle.context->Unmap(state->staging.get(), 0);

    WindowsWgcFrame output;
    output.bgra = bgra;
    output.size = crop.size();
    output.stride = rowBytes;
    output.timestampMs = state->elapsed.isValid() ? state->elapsed.elapsed() : 0;
    return output;
}

/**
 * 【录制】【Windows采集】处理 WGC FrameArrived 回调。
 * @param weakState 弱引用采集状态。
 * @param sender WGC 帧池。
 * @return 无返回值。
 */
void handleFrameArrived(const std::weak_ptr<StreamState> &weakState,
                        const Capture::Direct3D11CaptureFramePool &sender)
{
    const std::shared_ptr<StreamState> state = weakState.lock();
    if (!state || !state->running) {
        return;
    }

    try {
        const Capture::Direct3D11CaptureFrame frame = sender.TryGetNextFrame();
        if (!frame) {
            reportStreamError(state, QStringLiteral("Windows Graphics Capture frame was empty"));
            return;
        }
        // 写出队列繁忙时在纹理读回前丢弃帧，避免为将被丢弃的帧支付拷贝成本
        if (state->backpressure.load(std::memory_order_relaxed)) {
            return;
        }

        QString error;
        std::optional<WindowsWgcFrame> bgra = readFrameToBgra(state, frame, &error);
        if (!bgra) {
            reportStreamError(state,
                              error.isEmpty()
                                  ? QStringLiteral("Windows Graphics Capture frame read failed")
                                  : error);
            return;
        }

        const auto callback = state->onFrame;
        if (callback && state->running) {
            callback(std::move(*bgra));
        }
    } catch (const winrt::hresult_error &exception) {
        reportStreamError(state,
                          winrtError(QStringLiteral("Windows Graphics Capture frame callback failed"),
                                     exception));
    }
}

}  // namespace

class WindowsWgcStream::Private {
public:
    /**
     * 【录制】【Windows采集】启动 WGC 长生命周期采集。
     * @param request 捕获请求。
     * @param onFrame 帧回调。
     * @param onError 错误回调。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(const CaptureRequest &request,
               FrameCallback onFrame,
               ErrorCallback onError,
               QString *error)
    {
        stop();
        if (error) {
            error->clear();
        }
        if (request.allOutputs) {
            if (error) {
                *error = QStringLiteral("Windows Graphics Capture stream does not support all-outputs recording");
            }
            return false;
        }
        if (!windowsWgcStreamSupported(error)) {
            return false;
        }

        try {
            ensureWinrtApartment();
            std::optional<D3DDeviceBundle> bundle = createD3DDevice(error);
            if (!bundle) {
                return false;
            }

            QScreen *screen = screenForCaptureRequest(request);
            if (!screen) {
                if (error) {
                    *error = QStringLiteral("no screen available for Windows Graphics Capture");
                }
                return false;
            }
            HMONITOR monitor = monitorForScreen(screen);
            if (!monitor) {
                if (error) {
                    *error = QStringLiteral("failed to resolve native monitor for Windows Graphics Capture");
                }
                return false;
            }

            m_item = graphicsCaptureItemForMonitor(monitor);
            const auto itemSize = m_item.Size();
            if (itemSize.Width <= 0 || itemSize.Height <= 0) {
                if (error) {
                    *error = QStringLiteral("Windows Graphics Capture item has invalid dimensions");
                }
                return false;
            }

            const QSize frameSize(itemSize.Width, itemSize.Height);
            const QRect crop = cropRectForRequest(request, screen->geometry(), frameSize);
            m_state = std::make_shared<StreamState>();
            m_state->bundle = std::move(*bundle);
            m_state->onFrame = std::move(onFrame);
            m_state->onError = std::move(onError);
            m_state->cropPixels = crop;
            m_state->outputSize = crop.size();
            m_state->elapsed.start();
            m_state->running = true;

            m_framePool = Capture::Direct3D11CaptureFramePool::CreateFreeThreaded(
                m_state->bundle.winrtDevice,
                DirectX::DirectXPixelFormat::B8G8R8A8UIntNormalized,
                3,
                itemSize);
            m_session = m_framePool.CreateCaptureSession(m_item);

            QString cursorError;
            if (!configureCursorCapture(m_session, request.includeCursor, &cursorError)) {
                stop();
                if (error) {
                    *error = cursorError.isEmpty()
                        ? QStringLiteral("failed to configure Windows Graphics Capture cursor")
                        : cursorError;
                }
                return false;
            }
            requestBorderlessCapture(m_session);

            const std::weak_ptr<StreamState> weakState = m_state;
            m_frameToken = m_framePool.FrameArrived(
                [weakState](const Capture::Direct3D11CaptureFramePool &sender,
                            const winrt::Windows::Foundation::IInspectable &) {
                    handleFrameArrived(weakState, sender);
                });
            m_hasFrameToken = true;
            m_session.StartCapture();

            markshot::debugLog("recording",
                               "【录制】【Windows采集】started mode=wgc-stream geometry=%d,%d %dx%d frame=%dx%d crop=%d,%d %dx%d",
                               request.sourceGeometry.x(),
                               request.sourceGeometry.y(),
                               request.sourceGeometry.width(),
                               request.sourceGeometry.height(),
                               frameSize.width(),
                               frameSize.height(),
                               crop.x(),
                               crop.y(),
                               crop.width(),
                               crop.height());
            return true;
        } catch (const winrt::hresult_error &exception) {
            stop();
            if (error) {
                *error = winrtError(QStringLiteral("Windows Graphics Capture stream failed"), exception);
            }
        }
        return false;
    }

    /**
     * 【录制】【Windows采集】停止 WGC 长生命周期采集。
     * @return 无返回值。
     */
    void stop()
    {
        if (m_state) {
            m_state->running = false;
        }
        try {
            if (m_framePool && m_hasFrameToken) {
                m_framePool.FrameArrived(m_frameToken);
            }
            m_hasFrameToken = false;
            if (m_session) {
                m_session.Close();
            }
            if (m_framePool) {
                m_framePool.Close();
            }
        } catch (const winrt::hresult_error &exception) {
            markshot::debugLog("recording",
                               "【录制】【Windows采集】stop failed error=%s",
                               winrtError(QStringLiteral("stopping Windows Graphics Capture stream failed"),
                                          exception)
                                   .toUtf8()
                                   .constData());
        }
        m_session = nullptr;
        m_framePool = nullptr;
        m_item = nullptr;
        m_state.reset();
    }

    /**
     * 【录制】【Windows采集】设置写出背压状态。
     * @param active 下游写出队列繁忙时为 true。
     * @return 无返回值。
     */
    void setBackpressure(bool active)
    {
        if (m_state) {
            m_state->backpressure.store(active, std::memory_order_relaxed);
        }
    }

private:
    std::shared_ptr<StreamState> m_state;
    Capture::GraphicsCaptureItem m_item{nullptr};
    Capture::Direct3D11CaptureFramePool m_framePool{nullptr};
    Capture::GraphicsCaptureSession m_session{nullptr};
    winrt::event_token m_frameToken{};
    bool m_hasFrameToken = false;
};

WindowsWgcStream::WindowsWgcStream()
    : d(std::make_unique<Private>())
{
}

WindowsWgcStream::~WindowsWgcStream()
{
    stop();
}

bool WindowsWgcStream::start(const CaptureRequest &request,
                             FrameCallback onFrame,
                             ErrorCallback onError,
                             QString *error)
{
    return d->start(request, std::move(onFrame), std::move(onError), error);
}

void WindowsWgcStream::stop()
{
    if (d) {
        d->stop();
    }
}

void WindowsWgcStream::setBackpressure(bool active)
{
    if (d) {
        d->setBackpressure(active);
    }
}

bool windowsWgcStreamSupported(QString *error)
{
    return wgc_support::windowsWgcSupported(error);
}

}  // namespace markshot::platform::windows

#else

namespace markshot::platform::windows {

class WindowsWgcStream::Private {
};

WindowsWgcStream::WindowsWgcStream()
    : d(std::make_unique<Private>())
{
}

WindowsWgcStream::~WindowsWgcStream() = default;

bool WindowsWgcStream::start(const CaptureRequest &request,
                             FrameCallback onFrame,
                             ErrorCallback onError,
                             QString *error)
{
    Q_UNUSED(request)
    Q_UNUSED(onFrame)
    Q_UNUSED(onError)
    if (error) {
        *error = QStringLiteral("Windows Graphics Capture stream is only available on Windows");
    }
    return false;
}

void WindowsWgcStream::stop()
{
}

void WindowsWgcStream::setBackpressure(bool active)
{
    Q_UNUSED(active)
}

bool windowsWgcStreamSupported(QString *error)
{
    if (error) {
        *error = QStringLiteral("Windows Graphics Capture stream is only available on Windows");
    }
    return false;
}

}  // namespace markshot::platform::windows

#endif
