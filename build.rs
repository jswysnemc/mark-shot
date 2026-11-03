use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

// Dawn functions called by ORT. If ORT starts calling a new one, the linker will re-add libwebgpu_dawn.so
// to NEEDED, which will be caught by the the_local_engine_never_touches_the_gpu test
const DAWN: &[&str] = &[
    "wgpuAdapterGetLimits",
    "wgpuAdapterHasFeature",
    "wgpuAdapterInfoFreeMembers",
    "wgpuAdapterPropertiesSubgroupMatrixConfigsFreeMembers",
    "wgpuAdapterRelease",
    "wgpuAdapterRequestDevice",
    "wgpuBindGroupLayoutRelease",
    "wgpuBindGroupRelease",
    "wgpuBufferAddRef",
    "wgpuBufferDestroy",
    "wgpuBufferGetConstMappedRange",
    "wgpuBufferGetMappedRange",
    "wgpuBufferGetMapState",
    "wgpuBufferGetSize",
    "wgpuBufferGetUsage",
    "wgpuBufferMapAsync",
    "wgpuBufferRelease",
    "wgpuBufferUnmap",
    "wgpuCommandBufferRelease",
    "wgpuCommandEncoderBeginComputePass",
    "wgpuCommandEncoderClearBuffer",
    "wgpuCommandEncoderCopyBufferToBuffer",
    "wgpuCommandEncoderFinish",
    "wgpuCommandEncoderRelease",
    "wgpuCommandEncoderResolveQuerySet",
    "wgpuComputePassEncoderDispatchWorkgroups",
    "wgpuComputePassEncoderDispatchWorkgroupsIndirect",
    "wgpuComputePassEncoderEnd",
    "wgpuComputePassEncoderRelease",
    "wgpuComputePassEncoderSetBindGroup",
    "wgpuComputePassEncoderSetPipeline",
    "wgpuComputePassEncoderWriteTimestamp",
    "wgpuComputePipelineAddRef",
    "wgpuComputePipelineGetBindGroupLayout",
    "wgpuComputePipelineRelease",
    "wgpuCreateInstance",
    "wgpuDeviceAddRef",
    "wgpuDeviceCreateBindGroup",
    "wgpuDeviceCreateBuffer",
    "wgpuDeviceCreateCommandEncoder",
    "wgpuDeviceCreateComputePipelineAsync",
    "wgpuDeviceCreateQuerySet",
    "wgpuDeviceCreateShaderModule",
    "wgpuDeviceGetAdapterInfo",
    "wgpuDeviceGetFeatures",
    "wgpuDeviceGetLimits",
    "wgpuDeviceGetQueue",
    "wgpuDevicePopErrorScope",
    "wgpuDevicePushErrorScope",
    "wgpuDeviceRelease",
    "wgpuInstanceAddRef",
    "wgpuInstanceRelease",
    "wgpuInstanceRequestAdapter",
    "wgpuInstanceWaitAny",
    "wgpuPipelineLayoutRelease",
    "wgpuQuerySetAddRef",
    "wgpuQuerySetRelease",
    "wgpuQueueRelease",
    "wgpuQueueSubmit",
    "wgpuQueueWriteBuffer",
    "wgpuShaderModuleAddRef",
    "wgpuShaderModuleRelease",
    "wgpuSupportedFeaturesFreeMembers",
    "wgpuSurfaceRelease",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_OCR_GPU").is_none() {
        return;
    }
    let dist = dawn_dir();
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("x86_64") {
        // Without Dawn enabled, ORT uses dynamic stubs.
        // Dawn is lazily loaded via dlopen only for GPU execution (ocr/dawn.rs)
        if let Some(dir) = &dist {
            println!("cargo:rustc-env=MARK_SHOT_DAWN_DIST={}", dir.display());
        }
        write_trampolines();
        return;
    }
    // The GPU dist links libwebgpu_dawn.so dynamically and the binary does not
    // start without it. Look next to the binary first, so a packaged copy works,
    // then in the ort cache this build took the library from.
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    match dist {
        Some(dir) => println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display()),
        None => println!(
            "cargo:warning=libwebgpu_dawn.so not found in the ort cache; \
             an installed binary may need LD_LIBRARY_PATH"
        ),
    }
}

fn write_trampolines() {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let mut asm = String::from(".section .text.mark_shot_dawn,\"ax\",@progbits\n");
    let mut names = format!("pub const NAMES: [&std::ffi::CStr; {}] = [\n", DAWN.len());
    for (i, name) in DAWN.iter().enumerate() {
        let _ = write!(
            asm,
            ".p2align 4\n.globl {name}\n.hidden {name}\n.type {name},@function\n{name}:\n    jmp qword ptr [rip + mark_shot_dawn_slots + {}]\n",
            i * 8
        );
        let _ = writeln!(names, "    c\"{name}\",");
    }
    asm.push_str(".section .data.mark_shot_dawn,\"aw\",@progbits\n.p2align 3\n.globl mark_shot_dawn_slots\n.hidden mark_shot_dawn_slots\nmark_shot_dawn_slots:\n");
    for _ in DAWN {
        asm.push_str("    .quad mark_shot_dawn_missing\n");
    }
    names.push_str("];\n");
    fs::write(out.join("dawn.s"), asm).expect("write dawn.s");
    fs::write(out.join("dawn_names.rs"), names).expect("write dawn_names.rs");
}

fn dawn_dir() -> Option<PathBuf> {
    let target = std::env::var("TARGET").ok()?;
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    let dists = cache.join("ort.pyke.io").join("dfbin").join(target);
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(dists).ok()?.filter_map(Result::ok) {
        let dir = entry.path();
        let library = dir.join("libwebgpu_dawn.so");
        let Ok(stamp) = fs::metadata(&library).and_then(|meta| meta.modified()) else {
            continue;
        };
        if newest.as_ref().is_none_or(|(before, _)| stamp > *before) {
            newest = Some((stamp, dir));
        }
    }
    newest.map(|(_, dir)| dir)
}
