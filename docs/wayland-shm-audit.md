# Wayland shared memory fix audit

**English** | [简体中文](zh-CN/wayland-shm-audit.md)

On 2026-11-19, commit `e2153b7` was audited against [Issue #175](https://github.com/jswysnemc/mark-shot/issues/175). The allocation migration covers the presentation and capture paths inspected, but two existing configure callbacks still bypassed buffer lifecycle handling. This follow-up fixes those callers and adds protocol regression tests. The issue remains open.

## Allocation and release coverage

| Path | Allocation and release | Finding |
| --- | --- | --- |
| Capture overlay, frozen background, scrolling subsurfaces | Local `SlotPool`; `attach_to` registers active use | Migrated; probe and scroll parent configure callbacks had the omissions described below |
| Pins, previews, menus, native windows | Local `SlotPool`; dropped active buffers wait for `wl_buffer.release` | No remaining Smithay shared memory pool was found in these paths |
| Native screencopy and image-copy capture | Local `RawPool`; capture completion destroys buffers and frame protocol objects | Uses the same file creation entry point without a shrink seal |
| Wayland recording software buffers | `tempfile::tempfile()` and an independent `wl_shm.create_pool`; completed frames are read before reuse, and the owner destroys the buffer on exit | Ordinary temporary files do not satisfy the shrink-seal requirement; this is not a missed sealed memfd |
| Cursor | `wp_cursor_shape` protocol | No client cursor theme pixel pool requires migration |

`RawPool` passes only `MFD_CLOEXEC`. Linux automatically sets `F_SEAL_SEAL` on that memfd, omits `F_SEAL_SHRINK`, and permanently rejects adding a shrink seal. This prevents satisfying the `F_SEAL_SHRINK` prerequisite for the UDMABUF import path. The term “unsealed” does not mean that `F_GET_SEALS` must return zero.

The local active-buffer count, additional logical slot reference, deferred destruction and connection shutdown cleanup were compared with the locked Smithay Client Toolkit 0.21.1 implementation. No new defect in this bookkeeping was confirmed. Protocol tests also inspect the descriptor received by the server, real release events and destruction requests actually sent by the client.

## Corrected callers

1. The monitor probe in `overlay/state/layer.rs` attached a raw `wl_buffer` without calling `activate`. The client consequently treated submitted pixels as writable, and dropping the owner immediately destroyed the buffer and returned its slot. It now uses `attach_to`, keeps the slot active until a real release event, and reports attachment errors through the configure result.
2. The scroll parent callback in `overlay/state/xdg.rs` first called `configure_surface`, which already initialized and attached a buffer for the configured size. The callback then cleared and directly attached the same buffer again. Initial configuration attached twice, and repeated configuration could submit a buffer that was still active. It now commits only window geometry; `configure_surface` owns pixel allocation and attachment.

Both callers predate `e2153b7`; that commit retained them while replacing the memory pools. The follow-up also removes the raw protocol buffer getters from `ShmBuffer` and slot `Buffer`, preventing presentation callers from bypassing active-use registration through these interfaces.

## Regression validation

Eight new automated tests establish real protocol connections using `UnixStream::pair()` and a private `wayland-server`. The server sends configure and release events under test control. It does not connect to the user's desktop, use a graphics driver, or require a compositor process. Shell protocol handling and shared memory handling are in separate files. New dependencies are limited to Linux tests.

Before the fix, `cargo test --lib shm_lifecycle_tests -- --nocapture` failed all three configure regressions:

- The submitted probe buffer reported `active()` as false.
- The first scroll parent configure recorded attachments `[0, 0]` instead of `[0]`.
- Scroll parent resizing recorded `[0, 0, 1, 1]` instead of `[0, 1]`.

Five pool lifecycle tests already passed before the caller fixes. They cover restored write access after release, deferred destruction of dropped active buffers, release of every buffer sharing a slot, connection shutdown cleanup, and 32 consecutive frames at the reported `2561 × 1440` size. The consecutive-frame test checks that pool growth does not add a shrink seal and that released frames reuse capacity and offsets.

All eight new regressions pass after the fix. Run them separately with:

```sh
cargo test --lib shm_lifecycle_tests
cargo test --lib shm_pool::lifecycle_tests
```

Workspace testing passed 1765 tests with zero failures; 46 tests retain their existing network, model or platform ignore conditions. Linux workspace Clippy for all targets (`-D warnings`), Rust formatting and diff whitespace checks passed.

## Issue status and limits

These results establish that the application buffer lifecycle omissions are corrected. They do not prove that the kernel orphan pins reported in Issue #175 have stopped. The probe and scroll parent caller errors also do not establish an explanation for the reported increase of 31 to 53 full-screen frames per session.

The reported Fedora 44, KWin, kernel `7.0.13-200.fc44.x86_64` and AMD Radeon 680M combination has not been retested. The issue remains open. A build containing the fixes still needs measurements with `scripts/check_foll_pin.py` across cancel, copy, pin and process exit. Private protocol tests, other compositors and idle-command calibration cannot replace that environment's acquired/released counter evidence.
