//! 【识别运行库】【发行清单】固定官方产物与内容校验，下载地址不随远端最新版本变化

use crate::ocr::models::Asset;

pub const VERSION: &str = "1.28.0";
pub const BASE_URL: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.28.0";

#[derive(Clone, Copy)]
pub struct Distribution {
    pub archive: Asset,
    pub library: Asset,
    pub shared: Asset,
    pub platform: &'static str,
}

/// 【识别运行库】【平台选择】无参数，返回当前平台的官方 CPU 运行库；没有验证过的架构返回空值
pub fn current() -> Option<Distribution> {
    for (
        platform,
        archive,
        bytes,
        digest,
        library_bytes,
        library_digest,
        shared_bytes,
        shared_digest,
    ) in [
        (
            "linux-x64",
            "onnxruntime-linux-x64-1.28.0.tgz",
            9_125_960,
            "a3e1b79d7bb1bf09696ce675f49e4064e6c81f6202b8225624fff0e93f8d6407",
            24_268_848,
            "1461ef7cc3d9e49982591721683cc3e3a55580aeca9a5254e7aac47b75ee4bab",
            14_632,
            "086ec1d5388f64153d9c63470d126693db9a182c8ce236d3a1119068471b8a0d",
        ),
        (
            "linux-aarch64",
            "onnxruntime-linux-aarch64-1.28.0.tgz",
            8_116_278,
            "e15ff8b5d85afe6c144d97c6fd432254bf76a219daaf17658087d6ecb3e8f0bb",
            20_591_712,
            "f1ec1a08eb99bd6e5401340f0a2b101381bf4694415480291dc13bcaa30f9ec7",
            198_792,
            "3b6be288fbfb7dff8770d08a23defdde18e8f7e0f5a2b344a0e5e238c999ea88",
        ),
        (
            "win-x64",
            "onnxruntime-win-x64-1.28.0.zip",
            78_796_801,
            "abef733dacbe2f571547a7150b479b5cb9cc0df22f96c24983a42cadb1b4f8bc",
            15_809_848,
            "18370c375f07357fa5874344a9d9ac17e6b6fe1eb18b1dd209d79483b4470257",
            21_856,
            "599629fa643707defe9156140ae5edd73531f221aa97b7585b1c9bb0a93586f8",
        ),
    ] {
        let matches = match platform {
            "linux-x64" => cfg!(all(target_os = "linux", target_arch = "x86_64")),
            "linux-aarch64" => cfg!(all(target_os = "linux", target_arch = "aarch64")),
            "win-x64" => cfg!(all(target_os = "windows", target_arch = "x86_64")),
            _ => false,
        };
        if matches {
            return Some(Distribution {
                platform,
                archive: Asset {
                    file: archive,
                    size: bytes,
                    sha256: digest,
                },
                library: Asset {
                    file: if cfg!(target_os = "windows") {
                        "onnxruntime.dll"
                    } else {
                        "libonnxruntime.so.1.28.0"
                    },
                    size: library_bytes,
                    sha256: library_digest,
                },
                shared: Asset {
                    file: if cfg!(target_os = "windows") {
                        "onnxruntime_providers_shared.dll"
                    } else {
                        "libonnxruntime_providers_shared.so"
                    },
                    size: shared_bytes,
                    sha256: shared_digest,
                },
            });
        }
    }
    None
}
