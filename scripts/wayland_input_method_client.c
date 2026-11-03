/* 【输入法测试】【协议客户端】仅在私有合成器中提供 input-method-v2，不进入发布程序 */
#define _POSIX_C_SOURCE 200809L
#include <poll.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <wayland-client.h>
#include "input-method-client.h"

struct client {
    struct wl_display *display;
    struct wl_seat *seat;
    struct zwp_input_method_manager_v2 *manager;
    struct zwp_input_method_v2 *method;
    uint32_t serial, cursor, anchor, hint, purpose;
    char *text;
    bool active, unavailable;
};

/** 【输入法测试】【激活接收】按 data 重置上下文并标记激活，method 来自协议；无返回值 */
static void activate(void *data, struct zwp_input_method_v2 *method) {
    (void)method;
    struct client *client = data;
    client->active = true;
    client->cursor = client->anchor = 0;
    client->hint = client->purpose = 0;
    free(client->text);
    client->text = strdup("");
}

/** 【输入法测试】【停用接收】按 data 标记停用，method 来自协议；无返回值 */
static void deactivate(void *data, struct zwp_input_method_v2 *method) {
    (void)method;
    ((struct client *)data)->active = false;
}

/** 【输入法测试】【正文接收】将 text、cursor、anchor 保存到 data，method 来自协议；无返回值 */
static void surrounding(void *data, struct zwp_input_method_v2 *method, const char *text, uint32_t cursor, uint32_t anchor) {
    (void)method;
    struct client *client = data;
    free(client->text);
    client->text = strdup(text);
    client->cursor = cursor;
    client->anchor = anchor;
}

/** 【输入法测试】【原因接收】忽略 data、method、cause，完整保留协议监听器结构；无返回值 */
static void cause(void *data, struct zwp_input_method_v2 *method, uint32_t cause) {
    (void)data; (void)method; (void)cause;
}

/** 【输入法测试】【类型接收】将 hint、purpose 保存到 data，method 来自协议；字段用途随 done 一并记录，无返回值 */
static void content_type(void *data, struct zwp_input_method_v2 *method, uint32_t hint, uint32_t purpose) {
    (void)method;
    struct client *client = data;
    client->hint = hint;
    client->purpose = purpose;
}

/** 【输入法测试】【状态交付】递增 data 对象序号并输出当前状态，method 来自协议；无返回值，文字采用十六进制 */
static void done(void *data, struct zwp_input_method_v2 *method) {
    (void)method;
    struct client *client = data;
    client->serial++;
    printf("content\t%u\t%u\n", client->hint, client->purpose);
    printf("state\t%u\t%d\t%u\t%u\t", client->serial, client->active, client->cursor, client->anchor);
    for (const unsigned char *value = (const unsigned char *)client->text; value && *value; value++) {
        printf("%02x", *value);
    }
    puts("");
}

/** 【输入法测试】【占用接收】按 data 标记协议不可用，method 来自协议；无返回值 */
static void unavailable(void *data, struct zwp_input_method_v2 *method) {
    (void)method;
    ((struct client *)data)->unavailable = true;
    puts("unavailable");
}

static const struct zwp_input_method_v2_listener method_listener = {
    .activate = activate, .deactivate = deactivate, .surrounding_text = surrounding,
    .text_change_cause = cause, .content_type = content_type, .done = done, .unavailable = unavailable,
};

/** 【输入法测试】【全局绑定】按 name、interface、version 给 data 绑定首个 seat 和管理器，registry 来自协议；无返回值 */
static void global(void *data, struct wl_registry *registry, uint32_t name, const char *interface, uint32_t version) {
    (void)version;
    struct client *client = data;
    if (!strcmp(interface, "wl_seat") && !client->seat) {
        client->seat = wl_registry_bind(registry, name, &wl_seat_interface, 1);
    } else if (!strcmp(interface, "zwp_input_method_manager_v2")) {
        client->manager = wl_registry_bind(registry, name, &zwp_input_method_manager_v2_interface, 1);
    }
}

/** 【输入法测试】【全局移除】忽略 data、registry、name，运行器管理整个私有合成器生命周期；无返回值 */
static void global_remove(void *data, struct wl_registry *registry, uint32_t name) {
    (void)data; (void)registry; (void)name;
}

static const struct wl_registry_listener registry_listener = {.global = global, .global_remove = global_remove};

/** 【输入法测试】【十六进制解码】返回 hex 对应字节字符串；奇数长度、非法数字或内嵌空字符返回空值 */
static char *decode(const char *hex) {
    size_t length = strlen(hex);
    if (length % 2) { return NULL; }
    char *text = calloc(length / 2 + 1, 1);
    if (!text) { return NULL; }
    for (size_t index = 0; index < length; index += 2) {
        char part[3] = {hex[index], hex[index + 1], 0}, *end = NULL;
        long value = strtol(part, &end, 16);
        if (*end || !value) { free(text); return NULL; }
        text[index / 2] = (char)value;
    }
    return text;
}

/** 【输入法测试】【命令执行】按 line 给 client 暂存或发送事件，返回是否继续；非法命令终止本次测试 */
static bool command(struct client *client, char *line) {
    if (!strcmp(line, "quit")) { return false; }
    if (!strcmp(line, "send") || !strcmp(line, "stale")) {
        uint32_t serial = client->serial - !strcmp(line, "stale");
        zwp_input_method_v2_commit(client->method, serial);
        printf("sent\t%u\n", serial);
    } else if (!strncmp(line, "commit\t", 7)) {
        char *text = decode(line + 7);
        if (!text) { return false; }
        zwp_input_method_v2_commit_string(client->method, text);
        free(text);
        puts("ack");
    } else if (!strncmp(line, "preedit\t", 8)) {
        int begin, end;
        char *payload = strrchr(line, '\t');
        if (sscanf(line + 8, "%d\t%d", &begin, &end) != 2 || !payload) { return false; }
        char *text = decode(payload + 1);
        if (!text) { return false; }
        zwp_input_method_v2_set_preedit_string(client->method, text, begin, end);
        free(text);
        puts("ack");
    } else if (!strncmp(line, "delete\t", 7)) {
        unsigned int before, after;
        if (sscanf(line + 7, "%u\t%u", &before, &after) != 2) { return false; }
        zwp_input_method_v2_delete_surrounding_text(client->method, before, after);
        puts("ack");
    } else { return false; }
    wl_display_flush(client->display);
    return true;
}

/** 【输入法测试】【运行入口】连接环境指定的私有 Wayland，收发标准输入命令，返回连接或测试状态；无参数 */
int main(void) {
    setvbuf(stdout, NULL, _IOLBF, 0);
    struct client client = {0};
    client.text = strdup("");
    client.display = wl_display_connect(NULL);
    if (!client.display) { return 2; }
    struct wl_registry *registry = wl_display_get_registry(client.display);
    wl_registry_add_listener(registry, &registry_listener, &client);
    if (wl_display_roundtrip(client.display) < 0 || !client.manager || !client.seat) { return 3; }
    client.method = zwp_input_method_manager_v2_get_input_method(client.manager, client.seat);
    zwp_input_method_v2_add_listener(client.method, &method_listener, &client);
    if (wl_display_roundtrip(client.display) < 0 || client.unavailable) { return 4; }
    puts("ready");
    char input[16384];
    size_t used = 0;
    bool running = true;
    while (running && !client.unavailable) {
        while (wl_display_prepare_read(client.display) != 0) {
            if (wl_display_dispatch_pending(client.display) < 0) { return 5; }
        }
        wl_display_flush(client.display);
        struct pollfd fds[2] = {{wl_display_get_fd(client.display), POLLIN, 0}, {STDIN_FILENO, POLLIN, 0}};
        int result = poll(fds, 2, -1);
        if (result < 0) { wl_display_cancel_read(client.display); return 6; }
        if (fds[0].revents & POLLIN) {
            if (wl_display_read_events(client.display) < 0) { return 7; }
        } else { wl_display_cancel_read(client.display); }
        if (wl_display_dispatch_pending(client.display) < 0) { return 8; }
        if (fds[1].revents & (POLLIN | POLLHUP)) {
            ssize_t count = read(STDIN_FILENO, input + used, sizeof(input) - used - 1);
            if (count <= 0) { break; }
            used += (size_t)count;
            input[used] = 0;
            char *line = input, *end;
            while ((end = strchr(line, '\n'))) {
                *end = 0;
                if (!command(&client, line)) { running = false; break; }
                line = end + 1;
            }
            used -= (size_t)(line - input);
            memmove(input, line, used);
            if (used >= sizeof(input) - 1) { return 9; }
        }
    }
    zwp_input_method_v2_destroy(client.method);
    zwp_input_method_manager_v2_destroy(client.manager);
    wl_seat_destroy(client.seat);
    wl_registry_destroy(registry);
    wl_display_flush(client.display);
    wl_display_disconnect(client.display);
    free(client.text);
    return client.unavailable ? 4 : 0;
}
