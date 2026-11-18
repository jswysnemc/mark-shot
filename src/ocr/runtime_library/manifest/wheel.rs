//! 【识别运行库】【官方资源】固定 PyPI wheel 地址与摘要，只使用原生库和版权资料

use super::Distribution;
use crate::ocr::models::Asset;

/// 【识别运行库】【资源选择】无参数，返回当前平台官方 wheel 与内含原生库的完整校验清单，不安装或执行 Python 代码
pub(super) fn current() -> Option<Distribution> {
    let mut dist = super::legacy()?;
    let (base_url, archive, size, digest, library_size, library_digest, shared_size, shared_digest) =
        match dist.platform {
            "linux-x64" => (
                "https://files.pythonhosted.org/packages/03/88/9432428273356ad3c8aa01f52c1b3e7f53c4c0192748f41ad983872b436b",
                "onnxruntime-1.28.0-cp314-cp314-manylinux_2_27_x86_64.manylinux_2_28_x86_64.whl",
                19_214_924,
                "6afdc83f1317c136e92fc29f5ee9f058de59d87c0b22cee3fdbfbaa0ccc2098a",
                24_272_944,
                "aa4079d18f4ea7a5f3a94d80cd4bbe0f2740436626622d64d793803a20381083",
                14_632,
                "086ec1d5388f64153d9c63470d126693db9a182c8ce236d3a1119068471b8a0d",
            ),
            "linux-aarch64" => (
                "https://files.pythonhosted.org/packages/3f/e9/139180b4dd810329aaa42c238b4e6383c906202d98609ae29d66eb7c32b1",
                "onnxruntime-1.28.0-cp314-cp314-manylinux_2_27_aarch64.manylinux_2_28_aarch64.whl",
                17_051_950,
                "bc2565e487b4896fb988d6383577d875d958e071fc5f6c3550bd5d02ae98264b",
                20_657_248,
                "23a37c2994bc273e1d59a4f49af6cba5bf1cfbc29f04907ec27df375aba50a7d",
                198_792,
                "3b6be288fbfb7dff8770d08a23defdde18e8f7e0f5a2b344a0e5e238c999ea88",
            ),
            "win-x64" => (
                "https://files.pythonhosted.org/packages/bb/e2/6feb3a43517aaf2b1bf7e46897ba5eb81a29717f7d7901420614d5ee4653",
                "onnxruntime-1.28.0-cp314-cp314-win_amd64.whl",
                14_093_738,
                "f2a3b9e30ce880d4ca54999cb313569e36da4f62eefe25f87be18f43e9a3a4d5",
                17_766_752,
                "6788050502eeb1317e359c84339a6b7d67635e96e231794f4e608c1c00aae058",
                21_816,
                "380f1b47bfbc7164742f9c6bedcd76f43768b1062968cc4bd9974f721bca6894",
            ),
            _ => return None,
        };
    dist.base_url = base_url;
    dist.archive = Asset {
        file: archive,
        size,
        sha256: digest,
    };
    dist.library.size = library_size;
    dist.library.sha256 = library_digest;
    dist.shared.size = shared_size;
    dist.shared.sha256 = shared_digest;
    Some(dist)
}
