/* GPL-3.0-only: bundled GPU Screen Recorder integration */
#ifndef MARK_SHOT_GSR_CROP_H
#define MARK_SHOT_GSR_CROP_H

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

typedef struct { int x, y, width, height; } mark_shot_crop;

/** 【录制组件】【裁剪读取】从环境读取相对像素区域到 crop，未指定时置零，返回格式是否有效 */
static bool mark_shot_crop_read(mark_shot_crop *crop) {
    *crop = (mark_shot_crop){0};
    const char *value = getenv("MARK_SHOT_RECORDING_CROP");
    if(!value || !*value)
        return true;
    char tail;
    if(sscanf(value, "%d,%d,%d,%d%c", &crop->x, &crop->y, &crop->width, &crop->height, &tail) != 4)
        return false;
    return crop->x >= 0 && crop->y >= 0 && crop->width >= 2 && crop->height >= 2 && !(crop->width & 1) && !(crop->height & 1);
}

/** 【录制组件】【裁剪验证】按 crop、width、height 检查边界；无裁剪时允许完整源，返回是否可用 */
static bool mark_shot_crop_valid(const mark_shot_crop *crop, int width, int height) {
    return crop->width == 0 || ((int64_t)crop->x + crop->width <= width && (int64_t)crop->y + crop->height <= height);
}

#endif
