#pragma once

#include <onnxruntime_c_api.h>

namespace markshot::ocr_rapid {

/** 【OCR】【原生句柄】将 T 的寿命绑定到创建它的运行库释放函数，不允许复制或转移出推理作用域。 */
template <typename T>
class RapidOnnxHandle final {
public:
    using Release = void (ORT_API_CALL *)(T *);
    /** 使用 release 释放原生指针，初始为空。 */
    explicit RapidOnnxHandle(Release release) : m_release(release) {}
    /** 释放持有的指针，无参数和返回值。 */
    ~RapidOnnxHandle() { reset(); }
    RapidOnnxHandle(const RapidOnnxHandle &) = delete;
    RapidOnnxHandle &operator=(const RapidOnnxHandle &) = delete;

    /** 返回借用的原生指针，无参数。 */
    T *get() const { return m_value; }
    /** 先清理旧对象，再提供原生 API 输出位置，无参数。 */
    T **put() { reset(); return &m_value; }
    /** 释放旧对象并接管 value，省略参数时清空，无返回值。 */
    void reset(T *value = nullptr)
    {
        if (m_value) m_release(m_value);
        m_value = value;
    }

private:
    Release m_release;
    T *m_value = nullptr;
};

}  // namespace markshot::ocr_rapid
