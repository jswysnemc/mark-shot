#include <onnxruntime_c_api.h>

#include <cstdio>
#include <cstdlib>

namespace {

/** 【OCR测试】【原生线程】在线程退出时记录释放，验证运行库不能先于线程局部状态卸载。 */
struct ThreadState {
    /** 向测试指定的文件追加释放记录，无参数和返回值。 */
    ~ThreadState()
    {
        const char *path = std::getenv("MARK_SHOT_TEST_ORT_THREAD_TRACE");
        if (!path) return;
        if (FILE *file = std::fopen(path, "a")) {
            std::fputs("thread-ended\n", file);
            std::fclose(file);
        }
    }
};

/** 【OCR测试】【线程绑定】为调用线程创建原生状态并返回构建文字，无参数。 */
const char *ORT_API_CALL touchThread() noexcept
{
    thread_local ThreadState state;
    (void)state;
    return "mark-shot-lifetime-test";
}

/** 【OCR测试】【接口版本】仅向匹配 version 的调用方提供测试 API 表。 */
const OrtApi *ORT_API_CALL apiForVersion(uint32_t version) noexcept
{
    static const OrtApi api = [] {
        OrtApi table{};
        table.GetBuildInfoString = touchThread;
        return table;
    }();
    return version == ORT_API_VERSION ? &api : nullptr;
}

/** 【OCR测试】【版本文字】返回固定测试版本，无参数。 */
const char *ORT_API_CALL versionString() noexcept { return "mark-shot-lifetime-test"; }

}  // namespace

/** 【OCR测试】【运行库入口】导出只供寿命测试使用的 API 基表，无参数。 */
const OrtApiBase *ORT_API_CALL OrtGetApiBase() noexcept
{
    static const OrtApiBase base{apiForVersion, versionString};
    return &base;
}
