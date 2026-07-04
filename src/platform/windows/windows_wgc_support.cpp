#include "platform/windows/windows_wgc_support.h"

#if defined(Q_OS_WIN)

#include "debug_log.h"

#include <QGuiApplication>

#include <limits>

namespace markshot::platform::windows::wgc_support {
namespace {

namespace Metadata = winrt::Windows::Foundation::Metadata;

struct MonitorSearch {
    QScreen *screen = nullptr;
    HMONITOR monitor = nullptr;
};

/**
 * 【录制】【Windows采集】把 HRESULT 格式化为文本。
 * @param result HRESULT 错误码。
 * @return 错误码文本。
 */
QString hresultText(HRESULT result)
{
    return QStringLiteral("HRESULT 0x%1")
        .arg(static_cast<quint32>(result), 8, 16, QLatin1Char('0'));
}

/**
 * 【录制】【Windows采集】判断 WGC 会话是否支持隐藏边框。
 * @return 支持时返回 true。
 */
bool borderControlAvailable()
{
    static const bool available = [] {
        try {
            ensureWinrtApartment();
            return Metadata::ApiInformation::IsPropertyPresent(
                L"Windows.Graphics.Capture.GraphicsCaptureSession",
                L"IsBorderRequired");
        } catch (const winrt::hresult_error &) {
            return false;
        }
    }();
    return available;
}

/**
 * 【录制】【Windows采集】判断 WGC 会话是否支持鼠标控制。
 * @return 支持时返回 true。
 */
bool cursorControlAvailable()
{
    static const bool available = [] {
        try {
            ensureWinrtApartment();
            return Metadata::ApiInformation::IsPropertyPresent(
                L"Windows.Graphics.Capture.GraphicsCaptureSession",
                L"IsCursorCaptureEnabled");
        } catch (const winrt::hresult_error &) {
            return false;
        }
    }();
    return available;
}

/**
 * 【录制】【Windows采集】按指定驱动类型创建 D3D11 设备。
 * @param driverType D3D 驱动类型。
 * @param device 输出 D3D11 设备。
 * @param context 输出 D3D11 上下文。
 * @return HRESULT 调用结果。
 */
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

/**
 * 【录制】【Windows采集】枚举显示器时匹配 Qt 屏幕。
 * @param monitor 原生显示器句柄。
 * @param userData 查询状态。
 * @return 找到后返回 FALSE 停止枚举。
 */
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

}  // namespace

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

bool windowsWgcSupported(QString *error)
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

void requestBorderlessCapture(const Capture::GraphicsCaptureSession &session)
{
    if (!borderControlAvailable()) {
        markshot::debugLog("recording", "【录制】【Windows采集】border-control unsupported");
        return;
    }

    try {
        session.IsBorderRequired(false);
    } catch (const winrt::hresult_error &exception) {
        markshot::debugLog("recording",
                           "【录制】【Windows采集】border-control failed error=%s",
                           winrtError(QStringLiteral("setting borderless capture failed"), exception)
                               .toUtf8()
                               .constData());
    }
}

bool configureCursorCapture(const Capture::GraphicsCaptureSession &session,
                            bool includeCursor,
                            QString *error)
{
    if (!cursorControlAvailable()) {
        if (error) {
            *error = QStringLiteral("Windows Graphics Capture cursor control is not available");
        }
        return false;
    }

    try {
        session.IsCursorCaptureEnabled(includeCursor);
        return session.IsCursorCaptureEnabled() == includeCursor;
    } catch (const winrt::hresult_error &exception) {
        if (error) {
            *error = winrtError(QStringLiteral("setting cursor capture failed"), exception);
        }
    }
    return false;
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

#if QT_VERSION >= QT_VERSION_CHECK(6, 7, 0)
    if (auto *nativeScreen = screen->nativeInterface<QNativeInterface::QWindowsScreen>()) {
        if (HMONITOR monitor = nativeScreen->handle()) {
            return monitor;
        }
    }
#endif

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

QRect cropRectForRequest(const CaptureRequest &request,
                         const QRect &screenGeometry,
                         const QSize &frameSize)
{
    if (screenGeometry.isEmpty() || frameSize.isEmpty()) {
        return QRect(QPoint(0, 0), frameSize);
    }

    const QRect requested = request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()
        ? request.sourceGeometry.normalized()
        : screenGeometry;
    const QRect logicalCrop = requested.intersected(screenGeometry);
    if (logicalCrop.isEmpty()) {
        return QRect(QPoint(0, 0), frameSize);
    }

    const double scaleX = static_cast<double>(frameSize.width()) / screenGeometry.width();
    const double scaleY = static_cast<double>(frameSize.height()) / screenGeometry.height();
    QRect crop(qRound((logicalCrop.x() - screenGeometry.x()) * scaleX),
               qRound((logicalCrop.y() - screenGeometry.y()) * scaleY),
               qRound(logicalCrop.width() * scaleX),
               qRound(logicalCrop.height() * scaleY));
    crop = crop.intersected(QRect(QPoint(0, 0), frameSize));
    return crop.isEmpty() ? QRect(QPoint(0, 0), frameSize) : crop;
}

}  // namespace markshot::platform::windows::wgc_support

#endif
