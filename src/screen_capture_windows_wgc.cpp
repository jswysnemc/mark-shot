#include "screen_capture_internal.h"

#if defined(Q_OS_WIN)

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif

#ifdef signals
#define MARKSHOT_RESTORE_QT_SIGNALS_MACRO
#pragma push_macro("signals")
#undef signals
#endif
#ifdef slots
#define MARKSHOT_RESTORE_QT_SLOTS_MACRO
#pragma push_macro("slots")
#undef slots
#endif

#include <d3d11.h>
#include <dxgi1_2.h>
#include <windows.h>
#include <windows.graphics.capture.interop.h>
#include <windows.graphics.directx.direct3d11.interop.h>
#include <winrt/Windows.Foundation.Metadata.h>
#include <winrt/Windows.Graphics.Capture.h>
#include <winrt/Windows.Graphics.DirectX.Direct3D11.h>
#include <winrt/Windows.Graphics.DirectX.h>

#ifdef MARKSHOT_RESTORE_QT_SLOTS_MACRO
#pragma pop_macro("slots")
#undef MARKSHOT_RESTORE_QT_SLOTS_MACRO
#endif
#ifdef MARKSHOT_RESTORE_QT_SIGNALS_MACRO
#pragma pop_macro("signals")
#undef MARKSHOT_RESTORE_QT_SIGNALS_MACRO
#endif

#include <chrono>
#include <condition_variable>
#include <cstring>
#include <limits>
#include <memory>
#include <mutex>

namespace {

namespace Capture = winrt::Windows::Graphics::Capture;
namespace Direct3D11 = winrt::Windows::Graphics::DirectX::Direct3D11;
namespace DirectX = winrt::Windows::Graphics::DirectX;
namespace Metadata = winrt::Windows::Foundation::Metadata;

struct D3DDeviceBundle {
    winrt::com_ptr<ID3D11Device> device;
    winrt::com_ptr<ID3D11DeviceContext> context;
    Direct3D11::IDirect3DDevice winrtDevice{nullptr};
};

struct MonitorSearch {
    QScreen *screen = nullptr;
    HMONITOR monitor = nullptr;
};

struct FrameCaptureState {
    explicit FrameCaptureState(const D3DDeviceBundle &sourceBundle)
        : bundle(sourceBundle)
    {
    }

    D3DDeviceBundle bundle;
    std::mutex mutex;
    std::condition_variable condition;
    QImage captured;
    QString error;
    bool finished = false;
};

QString hresultText(HRESULT result)
{
    return QStringLiteral("HRESULT 0x%1")
        .arg(static_cast<quint32>(result), 8, 16, QLatin1Char('0'));
}

QString hresultError(const QString &context, HRESULT result)
{
    return QStringLiteral("%1 (%2)").arg(context, hresultText(result));
}

QString winrtError(const QString &context, const winrt::hresult_error &error)
{
    const QString message = QString::fromWCharArray(error.message().c_str());
    return message.isEmpty()
        ? hresultError(context, static_cast<HRESULT>(error.code()))
        : QStringLiteral("%1: %2 (%3)")
              .arg(context, message, hresultText(static_cast<HRESULT>(error.code())));
}

void ensureWinrtApartment()
{
    static thread_local bool attempted = false;
    if (attempted) {
        return;
    }
    attempted = true;

    try {
        winrt::init_apartment(winrt::apartment_type::multi_threaded);
    } catch (const winrt::hresult_error &error) {
        if (static_cast<HRESULT>(error.code()) != RPC_E_CHANGED_MODE) {
            throw;
        }
    }
}

bool windowsGraphicsCaptureSupported(QString *error)
{
    try {
        ensureWinrtApartment();
        if (!Metadata::ApiInformation::IsTypePresent(
                L"Windows.Graphics.Capture.GraphicsCaptureItem")) {
            if (error) {
                *error = QStringLiteral("Windows Graphics Capture is not available on this system");
            }
            return false;
        }
        if (!Capture::GraphicsCaptureSession::IsSupported()) {
            if (error) {
                *error = QStringLiteral("Windows Graphics Capture is not supported in this session");
            }
            return false;
        }
        return true;
    } catch (const winrt::hresult_error &exception) {
        if (error) {
            *error = winrtError(QStringLiteral("Windows Graphics Capture support check failed"),
                                exception);
        }
    }
    return false;
}

HRESULT createD3DDeviceWithDriver(D3D_DRIVER_TYPE driverType,
                                  winrt::com_ptr<ID3D11Device> *device,
                                  winrt::com_ptr<ID3D11DeviceContext> *context)
{
    static constexpr D3D_FEATURE_LEVEL kFeatureLevels[] = {
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
        D3D_FEATURE_LEVEL_10_0,
    };

    ID3D11Device *rawDevice = nullptr;
    ID3D11DeviceContext *rawContext = nullptr;
    D3D_FEATURE_LEVEL selectedFeatureLevel = D3D_FEATURE_LEVEL_11_0;
    HRESULT result = D3D11CreateDevice(nullptr,
                                       driverType,
                                       nullptr,
                                       D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                                       kFeatureLevels,
                                       ARRAYSIZE(kFeatureLevels),
                                       D3D11_SDK_VERSION,
                                       &rawDevice,
                                       &selectedFeatureLevel,
                                       &rawContext);
    if (result == E_INVALIDARG) {
        result = D3D11CreateDevice(nullptr,
                                   driverType,
                                   nullptr,
                                   D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                                   kFeatureLevels + 1,
                                   ARRAYSIZE(kFeatureLevels) - 1,
                                   D3D11_SDK_VERSION,
                                   &rawDevice,
                                   &selectedFeatureLevel,
                                   &rawContext);
    }

    if (SUCCEEDED(result)) {
        device->attach(rawDevice);
        context->attach(rawContext);
    }
    return result;
}

std::optional<D3DDeviceBundle> createD3DDevice(QString *error)
{
    D3DDeviceBundle bundle;
    HRESULT result = createD3DDeviceWithDriver(D3D_DRIVER_TYPE_HARDWARE,
                                               &bundle.device,
                                               &bundle.context);
    if (FAILED(result)) {
        result = createD3DDeviceWithDriver(D3D_DRIVER_TYPE_WARP,
                                           &bundle.device,
                                           &bundle.context);
    }
    if (FAILED(result)) {
        if (error) {
            *error = hresultError(QStringLiteral("failed to create a D3D11 device"), result);
        }
        return std::nullopt;
    }

    winrt::com_ptr<ID3D11Multithread> multithread;
    if (SUCCEEDED(bundle.device->QueryInterface(__uuidof(ID3D11Multithread),
                                                multithread.put_void()))) {
        multithread->SetMultithreadProtected(TRUE);
    }

    winrt::com_ptr<IDXGIDevice> dxgiDevice;
    result = bundle.device->QueryInterface(__uuidof(IDXGIDevice), dxgiDevice.put_void());
    if (FAILED(result)) {
        if (error) {
            *error = hresultError(QStringLiteral("failed to get DXGI device"), result);
        }
        return std::nullopt;
    }

    winrt::com_ptr<::IInspectable> inspectable;
    result = CreateDirect3D11DeviceFromDXGIDevice(dxgiDevice.get(), inspectable.put());
    if (FAILED(result)) {
        if (error) {
            *error = hresultError(QStringLiteral("failed to create WinRT D3D device"), result);
        }
        return std::nullopt;
    }

    bundle.winrtDevice = inspectable.as<Direct3D11::IDirect3DDevice>();
    return bundle;
}

BOOL CALLBACK findMonitorForScreen(HMONITOR monitor, HDC, LPRECT, LPARAM userData)
{
    auto *search = reinterpret_cast<MonitorSearch *>(userData);
    if (!search || !search->screen) {
        return TRUE;
    }

    MONITORINFOEXW monitorInfo = {};
    monitorInfo.cbSize = sizeof(MONITORINFOEXW);
    if (!GetMonitorInfoW(monitor, &monitorInfo)) {
        return TRUE;
    }

    const QString deviceName = QString::fromWCharArray(monitorInfo.szDevice);
    const QString screenName = search->screen->name();
    if (!screenName.isEmpty()
        && (screenName == deviceName
            || deviceName.endsWith(screenName)
            || screenName.endsWith(deviceName))) {
        search->monitor = monitor;
        return FALSE;
    }
    return TRUE;
}

QScreen *screenForCaptureRequest(const CaptureRequest &request)
{
    const QList<QScreen *> screens = QGuiApplication::screens();
    if (!request.preferredOutputName.isEmpty()) {
        for (QScreen *screen : screens) {
            if (screen && screen->name() == request.preferredOutputName) {
                return screen;
            }
        }
    }

    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        if (QScreen *screen = QGuiApplication::screenAt(request.sourceGeometry.center())) {
            return screen;
        }
    }

    return QGuiApplication::primaryScreen();
}

HMONITOR monitorForScreen(QScreen *screen)
{
    if (!screen) {
        return nullptr;
    }

    MonitorSearch search;
    search.screen = screen;
    EnumDisplayMonitors(nullptr, nullptr, findMonitorForScreen, reinterpret_cast<LPARAM>(&search));
    if (search.monitor) {
        return search.monitor;
    }

    const QPoint center = screen->geometry().center();
    const POINT point{center.x(), center.y()};
    return MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
}

Capture::GraphicsCaptureItem graphicsCaptureItemForMonitor(HMONITOR monitor)
{
    auto factory =
        winrt::get_activation_factory<Capture::GraphicsCaptureItem,
                                      IGraphicsCaptureItemInterop>();
    Capture::GraphicsCaptureItem item{nullptr};
    winrt::check_hresult(factory->CreateForMonitor(
        monitor,
        winrt::guid_of<Capture::GraphicsCaptureItem>(),
        winrt::put_abi(item)));
    return item;
}

QImage imageFromFrame(const Capture::Direct3D11CaptureFrame &frame,
                      const D3DDeviceBundle &bundle,
                      QString *error)
{
    const Direct3D11::IDirect3DSurface surface = frame.Surface();
    if (!surface) {
        if (error) {
            *error = QStringLiteral("Windows Graphics Capture returned an empty surface");
        }
        return {};
    }

    winrt::com_ptr<IDirect3DDxgiInterfaceAccess> access =
        surface.as<IDirect3DDxgiInterfaceAccess>();
    winrt::com_ptr<ID3D11Texture2D> texture;
    HRESULT result = access->GetInterface(__uuidof(ID3D11Texture2D), texture.put_void());
    if (FAILED(result)) {
        if (error) {
            *error = hresultError(QStringLiteral("failed to access capture texture"), result);
        }
        return {};
    }

    D3D11_TEXTURE2D_DESC desc = {};
    texture->GetDesc(&desc);
    if (desc.Width == 0 || desc.Height == 0
        || desc.Width > static_cast<UINT>(std::numeric_limits<int>::max())
        || desc.Height > static_cast<UINT>(std::numeric_limits<int>::max())) {
        if (error) {
            *error = QStringLiteral("Windows Graphics Capture returned invalid frame dimensions");
        }
        return {};
    }

    desc.BindFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    desc.MiscFlags = 0;
    desc.Usage = D3D11_USAGE_STAGING;

    winrt::com_ptr<ID3D11Texture2D> staging;
    result = bundle.device->CreateTexture2D(&desc, nullptr, staging.put());
    if (FAILED(result)) {
        if (error) {
            *error = hresultError(QStringLiteral("failed to create staging texture"), result);
        }
        return {};
    }

    bundle.context->CopyResource(staging.get(), texture.get());

    D3D11_MAPPED_SUBRESOURCE mapped = {};
    result = bundle.context->Map(staging.get(), 0, D3D11_MAP_READ, 0, &mapped);
    if (FAILED(result)) {
        if (error) {
            *error = hresultError(QStringLiteral("failed to map staging texture"), result);
        }
        return {};
    }

    const int width = static_cast<int>(desc.Width);
    const int height = static_cast<int>(desc.Height);
    QImage image(width, height, QImage::Format_ARGB32_Premultiplied);
    if (!image.isNull()) {
        const int rowBytes = width * 4;
        for (int y = 0; y < height; ++y) {
            const auto *source = static_cast<const uchar *>(mapped.pData) + mapped.RowPitch * y;
            std::memcpy(image.scanLine(y), source, static_cast<std::size_t>(rowBytes));
        }
    }

    bundle.context->Unmap(staging.get(), 0);
    if (image.isNull() && error) {
        *error = QStringLiteral("failed to allocate Windows Graphics Capture image");
    }
    return image;
}

QImage captureItemFrame(const Capture::GraphicsCaptureItem &item,
                        const D3DDeviceBundle &bundle,
                        QString *error)
{
    const auto size = item.Size();
    if (size.Width <= 0 || size.Height <= 0) {
        if (error) {
            *error = QStringLiteral("Windows Graphics Capture item has invalid dimensions");
        }
        return {};
    }

    try {
        Capture::Direct3D11CaptureFramePool framePool =
            Capture::Direct3D11CaptureFramePool::CreateFreeThreaded(
                bundle.winrtDevice,
                DirectX::DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                size);
        Capture::GraphicsCaptureSession session = framePool.CreateCaptureSession(item);
        const auto state = std::make_shared<FrameCaptureState>(bundle);

        const winrt::event_token token = framePool.FrameArrived(
            [state](const Capture::Direct3D11CaptureFramePool &sender, const winrt::Windows::Foundation::IInspectable &) {
                std::lock_guard<std::mutex> lock(state->mutex);
                if (state->finished) {
                    return;
                }

                try {
                    const Capture::Direct3D11CaptureFrame frame = sender.TryGetNextFrame();
                    if (frame) {
                        state->captured = imageFromFrame(frame, state->bundle, &state->error);
                    } else {
                        state->error = QStringLiteral("Windows Graphics Capture frame was empty");
                    }
                } catch (const winrt::hresult_error &exception) {
                    state->error = winrtError(QStringLiteral("Windows Graphics Capture frame read failed"),
                                              exception);
                }

                state->finished = true;
                state->condition.notify_one();
            });

        session.StartCapture();

        {
            std::unique_lock<std::mutex> lock(state->mutex);
            if (!state->condition.wait_for(lock, std::chrono::seconds(3), [&state] {
                    return state->finished;
                })) {
                state->error = QStringLiteral("Windows Graphics Capture timed out waiting for a frame");
                state->finished = true;
            }
        }

        framePool.FrameArrived(token);
        session.Close();
        framePool.Close();

        QImage captured;
        QString captureError;
        {
            std::lock_guard<std::mutex> lock(state->mutex);
            captured = state->captured;
            captureError = state->error;
        }

        if (captured.isNull() && error) {
            *error = captureError.isEmpty()
                ? QStringLiteral("Windows Graphics Capture returned no frame")
                : captureError;
        }
        return captured;
    } catch (const winrt::hresult_error &exception) {
        if (error) {
            *error = winrtError(QStringLiteral("Windows Graphics Capture failed"), exception);
        }
    }
    return {};
}

CaptureResult captureMonitorWithWindowsGraphicsCapture(QScreen *screen,
                                                       const D3DDeviceBundle &bundle)
{
    if (!screen) {
        return {{}, QStringLiteral("no screen available for Windows Graphics Capture"), {}, {}};
    }

    HMONITOR monitor = monitorForScreen(screen);
    if (!monitor) {
        return {{}, QStringLiteral("failed to resolve native monitor for Windows Graphics Capture"), {}, screen->geometry()};
    }

    try {
        const Capture::GraphicsCaptureItem item = graphicsCaptureItemForMonitor(monitor);
        QString error;
        QImage image = captureItemFrame(item, bundle, &error);
        if (image.isNull()) {
            return {{},
                    error.isEmpty() ? QStringLiteral("Windows Graphics Capture returned no frame") : error,
                    screen->name(),
                    screen->geometry()};
        }

        markshot::debugLog("capture",
                           "windows-graphics-capture monitor=%s geom=%d,%d %dx%d frame=%dx%d",
                           screen->name().toUtf8().constData(),
                           screen->geometry().x(), screen->geometry().y(),
                           screen->geometry().width(), screen->geometry().height(),
                           image.width(), image.height());
        return {image.convertToFormat(QImage::Format_ARGB32_Premultiplied),
                {},
                screen->name(),
                screen->geometry()};
    } catch (const winrt::hresult_error &exception) {
        return {{},
                winrtError(QStringLiteral("failed to create Windows Graphics Capture item"), exception),
                screen->name(),
                screen->geometry()};
    }
}

CaptureResult captureAllScreensWithWindowsGraphicsCapture(const CaptureRequest &request,
                                                          const D3DDeviceBundle &bundle)
{
    const QRect frameGeometry = request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()
        ? request.sourceGeometry.normalized()
        : virtualScreensGeometry();
    if (frameGeometry.isEmpty()) {
        return {{}, QStringLiteral("no virtual screen geometry available for Windows Graphics Capture"), {}, {}};
    }

    QImage combined(frameGeometry.size(), QImage::Format_ARGB32_Premultiplied);
    combined.fill(Qt::transparent);

    QPainter painter(&combined);
    painter.setCompositionMode(QPainter::CompositionMode_Source);

    int capturedScreens = 0;
    QStringList errors;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (!screen) {
            continue;
        }

        const QRect screenGeometry = screen->geometry();
        const QRect overlap = screenGeometry.intersected(frameGeometry);
        if (overlap.isEmpty()) {
            continue;
        }

        CaptureResult capture = captureMonitorWithWindowsGraphicsCapture(screen, bundle);
        if (capture.image.isNull()) {
            if (!capture.error.isEmpty()) {
                errors.append(QStringLiteral("%1: %2").arg(screen->name(), capture.error));
            }
            continue;
        }

        const QRect sourceRect = markshot::capture::scaledCropRect(screenGeometry,
                                                                   overlap,
                                                                   capture.image.size());
        if (sourceRect.isEmpty()) {
            continue;
        }

        const QRect destinationRect(overlap.topLeft() - frameGeometry.topLeft(), overlap.size());
        painter.drawImage(destinationRect, capture.image, sourceRect);
        ++capturedScreens;
    }
    painter.end();

    if (capturedScreens == 0) {
        const QString error = errors.isEmpty()
            ? QStringLiteral("Windows Graphics Capture returned no usable screen frames")
            : errors.join(QLatin1Char('\n'));
        return {{}, error, {}, frameGeometry};
    }

    markshot::debugLog("capture",
                       "windows-graphics-capture-all screens=%d virtual_geom=%d,%d %dx%d result=%dx%d",
                       capturedScreens,
                       frameGeometry.x(), frameGeometry.y(),
                       frameGeometry.width(), frameGeometry.height(),
                       combined.width(), combined.height());
    return {combined, {}, {}, frameGeometry};
}

CaptureResult cropWindowsGraphicsCaptureFrame(CaptureResult capture,
                                              const CaptureRequest &request)
{
    if (capture.image.isNull()) {
        return capture;
    }

    if (!request.sourceGeometry.isValid() || request.sourceGeometry.isEmpty()) {
        return capture;
    }

    const QRect requested = request.sourceGeometry.normalized();
    const QRect overlap = requested.intersected(capture.sourceGeometry);
    if (overlap.isEmpty()) {
        return {{},
                QStringLiteral("Windows Graphics Capture does not cover requested geometry"),
                capture.outputName,
                request.sourceGeometry};
    }

    QImage cropped = markshot::capture::cropFrameToRequest(capture.image,
                                                           capture.sourceGeometry,
                                                           requested);
    if (cropped.isNull()) {
        return {{},
                QStringLiteral("Windows Graphics Capture local crop is empty"),
                capture.outputName,
                request.sourceGeometry};
    }

    const QSize frameSize = capture.image.size();
    const QRect frameGeometry = capture.sourceGeometry;
    capture.image = cropped.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    capture.sourceGeometry = overlap;
    markshot::debugLog("capture",
                       "windows-graphics-capture-crop frame=%dx%d frame_geom=%d,%d %dx%d "
                       "requested=%d,%d %dx%d overlap=%d,%d %dx%d result=%dx%d",
                       frameSize.width(), frameSize.height(),
                       frameGeometry.x(), frameGeometry.y(),
                       frameGeometry.width(), frameGeometry.height(),
                       requested.x(), requested.y(), requested.width(), requested.height(),
                       overlap.x(), overlap.y(), overlap.width(), overlap.height(),
                       capture.image.width(), capture.image.height());
    return capture;
}

} // namespace

CaptureResult captureWithWindowsGraphicsCapture(const CaptureRequest &request)
{
    QString error;
    if (!windowsGraphicsCaptureSupported(&error)) {
        return {{}, error, {}, request.sourceGeometry};
    }

    const std::optional<D3DDeviceBundle> bundle = createD3DDevice(&error);
    if (!bundle.has_value()) {
        return {{}, error, {}, request.sourceGeometry};
    }

    if (request.allOutputs) {
        return captureAllScreensWithWindowsGraphicsCapture(request, *bundle);
    }

    QScreen *screen = screenForCaptureRequest(request);
    CaptureResult capture = captureMonitorWithWindowsGraphicsCapture(screen, *bundle);
    return cropWindowsGraphicsCaptureFrame(std::move(capture), request);
}

#endif // Q_OS_WIN
