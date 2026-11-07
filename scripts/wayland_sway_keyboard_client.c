/** 【私有键盘】【持续设备】在同一 Wayland 虚拟键盘上使用完整 US 映射，标准输入提供键码和修饰键 */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>
#include <wayland-client.h>
#include <xkbcommon/xkbcommon.h>
#include "virtual-keyboard-client.h"

struct device {
    struct wl_display *display;
    struct wl_seat *seat;
    struct zwp_virtual_keyboard_manager_v1 *manager;
    struct zwp_virtual_keyboard_v1 *keyboard;
    uint32_t masks[4];
};

/** 【私有键盘】【协议绑定】按 data 中设备绑定 name、interface、version 对应座席和管理器，无返回值 */
static void global(void *data, struct wl_registry *registry, uint32_t name,
                   const char *interface, uint32_t version)
{
    struct device *device = data;
    if (!strcmp(interface, "wl_seat") && !device->seat) {
        device->seat = wl_registry_bind(registry, name, &wl_seat_interface, version < 7 ? version : 7);
    } else if (!strcmp(interface, "zwp_virtual_keyboard_manager_v1")) {
        device->manager = wl_registry_bind(registry, name, &zwp_virtual_keyboard_manager_v1_interface, 1);
    }
}

/** 【私有键盘】【对象移除】座席生命周期由本次连接管理，data、registry、name 不触发外部动作，无返回值 */
static void removed(void *data, struct wl_registry *registry, uint32_t name)
{
    (void)data;
    (void)registry;
    (void)name;
}

/** 【私有键盘】【有效映射】为 device 创建完整 US 映射并投递给合成器，返回成功标志 */
static int keymap(struct device *device)
{
    struct xkb_context *context = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
    struct xkb_rule_names names = {.layout = "us"};
    struct xkb_keymap *map = xkb_keymap_new_from_names(context, &names, XKB_KEYMAP_COMPILE_NO_FLAGS);
    if (!map) {
        xkb_context_unref(context);
        return 0;
    }
    const char *modifiers[] = {XKB_MOD_NAME_CTRL, XKB_MOD_NAME_SHIFT, XKB_MOD_NAME_ALT, XKB_MOD_NAME_LOGO};
    for (int index = 0; index < 4; ++index) {
        device->masks[index] = 1u << xkb_keymap_mod_get_index(map, modifiers[index]);
    }
    char *text = xkb_keymap_get_as_string(map, XKB_KEYMAP_FORMAT_TEXT_V1);
    size_t size = strlen(text) + 1;
    int fd = memfd_create("private-keyboard-map", MFD_CLOEXEC);
    int ok = fd >= 0 && write(fd, text, size) == (ssize_t)size;
    if (ok) {
        // 1. 【私有键盘】【标准映射】键盘连接保持存活，不依赖临时设备退出后合成器重新选择映射
        zwp_virtual_keyboard_v1_keymap(device->keyboard, WL_KEYBOARD_KEYMAP_FORMAT_XKB_V1, fd, size);
        zwp_virtual_keyboard_v1_modifiers(device->keyboard, 0, 0, 0, 0);
        ok = wl_display_roundtrip(device->display) >= 0;
    }
    if (fd >= 0) {
        close(fd);
    }
    free(text);
    xkb_keymap_unref(map);
    xkb_context_unref(context);
    return ok;
}

/** 【私有键盘】【按键时刻】返回单调时钟毫秒数，按 Wayland 无符号时间约定自然回绕，无参数 */
static uint32_t instant(void)
{
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (uint32_t)((uint64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000);
}

/** 【私有键盘】【修饰键状态】向 device 投递 flags 对应标准修饰键，返回连接状态，零值释放持有 */
static int modifiers(struct device *device, unsigned flags)
{
    uint32_t mask = 0;
    for (int index = 0; index < 4; ++index) {
        if (flags & (1u << index)) {
            mask |= device->masks[index];
        }
    }
    zwp_virtual_keyboard_v1_modifiers(device->keyboard, mask, 0, 0, 0);
    return wl_display_roundtrip(device->display) >= 0;
}

/** 【私有键盘】【完整按键】向 device 投递 code 与 flags 指定的按下、释放和修饰键复位，返回连接状态 */
static int key(struct device *device, unsigned code, unsigned flags)
{
    if (!modifiers(device, flags)) {
        return 0;
    }
    zwp_virtual_keyboard_v1_key(device->keyboard, instant(), code, WL_KEYBOARD_KEY_STATE_PRESSED);
    if (wl_display_roundtrip(device->display) < 0) {
        return 0;
    }
    struct timespec delay = {.tv_nsec = 25000000};
    nanosleep(&delay, NULL);
    zwp_virtual_keyboard_v1_key(device->keyboard, instant(), code, WL_KEYBOARD_KEY_STATE_RELEASED);
    zwp_virtual_keyboard_v1_modifiers(device->keyboard, 0, 0, 0, 0);
    return wl_display_roundtrip(device->display) >= 0;
}

/** 【私有键盘】【命令循环】连接当前私有显示器并读取有界键码命令，返回进程状态，无参数 */
int main(void)
{
    struct device device = {0};
    device.display = wl_display_connect(NULL);
    if (!device.display) {
        return 2;
    }
    struct wl_registry *registry = wl_display_get_registry(device.display);
    const struct wl_registry_listener listener = {.global = global, .global_remove = removed};
    wl_registry_add_listener(registry, &listener, &device);
    if (wl_display_roundtrip(device.display) < 0 || !device.seat || !device.manager) {
        return 3;
    }
    device.keyboard = zwp_virtual_keyboard_manager_v1_create_virtual_keyboard(device.manager, device.seat);
    if (!keymap(&device)) {
        return 4;
    }
    puts("ready");
    fflush(stdout);
    char command[64];
    while (fgets(command, sizeof(command), stdin)) {
        unsigned code, flags;
        int ok;
        if (sscanf(command, "modifiers %u", &flags) == 1) {
            ok = flags <= 15 && modifiers(&device, flags);
        } else {
            ok = sscanf(command, "%u %u", &code, &flags) == 2 && code <= 255 && flags <= 15 && key(&device, code, flags);
        }
        if (!ok) {
            return 5;
        }
        puts("done");
        fflush(stdout);
    }
    zwp_virtual_keyboard_v1_destroy(device.keyboard);
    zwp_virtual_keyboard_manager_v1_destroy(device.manager);
    wl_seat_release(device.seat);
    wl_registry_destroy(registry);
    wl_display_disconnect(device.display);
    return 0;
}
