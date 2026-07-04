#pragma once

#include "screen_capture.h"

#include <QRect>
#include <QScreen>
#include <QString>

#include <optional>

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

namespace markshot::platform::windows::wgc_support {

namespace Capture = winrt::Windows::Graphics::Capture;
namespace Direct3D11 = winrt::Windows::Graphics::DirectX::Direct3D11;
namespace Direct3D11Abi = ::Windows::Graphics::DirectX::Direct3D11;
namespace DirectX = winrt::Windows::Graphics::DirectX;

struct D3DDeviceBundle {
    winrt::com_ptr<ID3D11Device> device;
    winrt::com_ptr<ID3D11DeviceContext> context;
    Direct3D11::IDirect3DDevice winrtDevice{nullptr};
};

/**
 * 【录制】【Windows采集】拼接 Win32 调用错误。
 * @param context 调用上下文。
 * @param result HRESULT 错误码。
 * @return 错误文本。
 */
QString hresultError(const QString &context, HRESULT result);

/**
 * 【录制】【Windows采集】拼接 WinRT 异常错误。
 * @param context 调用上下文。
 * @param error WinRT 异常。
 * @return 错误文本。
 */
QString winrtError(const QString &context, const winrt::hresult_error &error);

/**
 * 【录制】【Windows采集】初始化当前线程的 WinRT apartment。
 * @return 无返回值。
 */
void ensureWinrtApartment();

/**
 * 【录制】【Windows采集】判断当前系统是否支持 WGC。
 * @param error 输出错误信息。
 * @return 支持时返回 true。
 */
bool windowsWgcSupported(QString *error);

/**
 * 【录制】【Windows采集】设置 WGC 捕获边框。
 * @param session WGC 会话。
 * @return 无返回值。
 */
void requestBorderlessCapture(const Capture::GraphicsCaptureSession &session);

/**
 * 【录制】【Windows采集】设置 WGC 鼠标捕获。
 * @param session WGC 会话。
 * @param includeCursor 是否包含鼠标。
 * @param error 输出错误信息。
 * @return 设置成功时返回 true。
 */
bool configureCursorCapture(const Capture::GraphicsCaptureSession &session,
                            bool includeCursor,
                            QString *error);

/**
 * 【录制】【Windows采集】创建 D3D11 与 WinRT 共享设备。
 * @param error 输出错误信息。
 * @return 创建成功时返回设备组合。
 */
std::optional<D3DDeviceBundle> createD3DDevice(QString *error);

/**
 * 【录制】【Windows采集】按录制请求选择 Qt 屏幕。
 * @param request 捕获请求。
 * @return 匹配到的屏幕。
 */
QScreen *screenForCaptureRequest(const CaptureRequest &request);

/**
 * 【录制】【Windows采集】把 Qt 屏幕转换为原生显示器句柄。
 * @param screen Qt 屏幕。
 * @return 原生显示器句柄。
 */
HMONITOR monitorForScreen(QScreen *screen);

/**
 * 【录制】【Windows采集】创建显示器对应的 WGC 捕获项。
 * @param monitor 原生显示器句柄。
 * @return WGC 捕获项。
 */
Capture::GraphicsCaptureItem graphicsCaptureItemForMonitor(HMONITOR monitor);

/**
 * 【录制】【Windows采集】按屏幕逻辑区域计算帧内像素裁剪区域。
 * @param request 捕获请求。
 * @param screenGeometry Qt 屏幕逻辑坐标。
 * @param frameSize WGC 帧像素尺寸。
 * @return 帧内像素裁剪区域。
 */
QRect cropRectForRequest(const CaptureRequest &request,
                         const QRect &screenGeometry,
                         const QSize &frameSize);

}  // namespace markshot::platform::windows::wgc_support

#endif
