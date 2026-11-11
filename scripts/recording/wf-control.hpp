// MIT: Mark Shot integration for wf-recorder
#pragma once
#include <atomic>
#include <csignal>
#include <ctime>
#include <thread>
#include <chrono>

static_assert(std::atomic<bool>::is_always_lock_free);
static_assert(std::atomic<int64_t>::is_always_lock_free);
inline std::atomic<bool> mark_shot_paused{false};
inline std::atomic<int64_t> mark_shot_pause_us{0};
inline int64_t mark_shot_pause_start = 0;

/** 【录制组件】【单调时间】无参数，返回 CLOCK_MONOTONIC 微秒值，用于暂停与限帧 */
inline int64_t mark_shot_now() {
    timespec value{};
    clock_gettime(CLOCK_MONOTONIC, &value);
    return int64_t(value.tv_sec) * 1000000 + value.tv_nsec / 1000;
}

/** 【录制组件】【暂停信号】按系统 signal 切换暂停，返回空值；仅主线程接收 SIGUSR2 */
inline void mark_shot_toggle(int) {
    if(mark_shot_paused) {
        mark_shot_pause_us.fetch_add(mark_shot_now() - mark_shot_pause_start);
        mark_shot_paused = 0;
    } else {
        mark_shot_pause_start = mark_shot_now();
        mark_shot_paused = 1;
    }
}
