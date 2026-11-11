"""为固定版本的独立录制组件补充 Portal GPU 裁剪与无特权 X11 捕获。"""

from pathlib import Path
import shutil
import sys


def replace(text: str, old: str, new: str) -> str:
    """按唯一 old 替换为 new，返回源码；上游上下文变化时停止构建。"""
    if text.count(old) != 1:
        raise ValueError(f"Unexpected GSR source context: {old[:100]}")
    return text.replace(old, new, 1)


def patch(root: Path) -> None:
    """按 root 修改独立组件源码，不修改主程序；补丁源码随产物一起分发。"""
    portal = root / "src/capture/portal.c"
    text = portal.read_text()
    text = replace(text, '    vec2i capture_size;', '    vec2i capture_size;\n    mark_shot_crop mark_shot_region;')
    marker = 'typedef struct {'
    offset = text.index(marker)
    text = text[:offset] + '#include "mark-shot-crop.h"\n\n' + text[offset:]
    text = replace(text, '\n    const int result = gsr_capture_portal_setup(self, capture_metadata->fps);', '''
    if(!mark_shot_crop_read(&self->mark_shot_region)) {
        gsr_log(GSR_LOG_LEVEL_ERROR, "【录制组件】【区域参数】Invalid crop");
        return -1;
    }
    const int result = gsr_capture_portal_setup(self, capture_metadata->fps);''')
    text = replace(text, '    if(self->params.output_resolution.x == 0 && self->params.output_resolution.y == 0) {', '''    if(!mark_shot_crop_valid(&self->mark_shot_region, self->capture_size.x, self->capture_size.y)) {
        gsr_log(GSR_LOG_LEVEL_ERROR, "【录制组件】【区域边界】Selected source does not contain crop");
        return -1;
    }
    if(self->mark_shot_region.width > 0) {
        capture_metadata->video_size = (vec2i){self->mark_shot_region.width, self->mark_shot_region.height};
    } else if(self->params.output_resolution.x == 0 && self->params.output_resolution.y == 0) {''')
    text = replace(text, '    const vec2i output_size = scale_keep_aspect_ratio(self->capture_size, capture_metadata->recording_size);', '''    const mark_shot_crop crop = self->mark_shot_region;
    if(!mark_shot_crop_valid(&crop, self->pipewire_data.region.width, self->pipewire_data.region.height)
        || (crop.width > 0 && self->pipewire_data.rotation != GSR_MONITOR_ROT_0)) {
        gsr_log(GSR_LOG_LEVEL_ERROR, "【录制组件】【区域边界】Source resized or rotated; restart recording");
        self->should_stop = true;
        self->stop_is_error = true;
        return -1;
    }
    const vec2i visible_size = crop.width > 0 ? (vec2i){crop.width, crop.height} : self->capture_size;
    const vec2i source_pos = {self->pipewire_data.region.x + crop.x, self->pipewire_data.region.y + crop.y};
    const vec2i source_size = crop.width > 0 ? visible_size : (vec2i){self->pipewire_data.region.width, self->pipewire_data.region.height};
    const vec2i output_size = scale_keep_aspect_ratio(visible_size, capture_metadata->recording_size);''')
    text = replace(text, '    // TODO: Handle region crop', '    // 1. 【录制组件】【GPU裁剪】区域只影响纹理坐标，不读回原始视频像素')
    text = replace(text, '(vec2i){self->pipewire_data.region.x, self->pipewire_data.region.y}, (vec2i){self->pipewire_data.region.width, self->pipewire_data.region.height}, actual_texture_size,', 'source_pos, source_size, actual_texture_size,')
    text = replace(text, 'self->capture_size.x == 0 ? 0 : (double)output_size.x / (double)self->capture_size.x', 'visible_size.x == 0 ? 0 : (double)output_size.x / (double)visible_size.x')
    text = replace(text, 'self->capture_size.y == 0 ? 0 : (double)output_size.y / (double)self->capture_size.y', 'visible_size.y == 0 ? 0 : (double)output_size.y / (double)visible_size.y')
    text = replace(text, 'self->pipewire_data.cursor_region.x - self->pipewire_data.region.x', 'self->pipewire_data.cursor_region.x - source_pos.x')
    text = replace(text, 'self->pipewire_data.cursor_region.y - self->pipewire_data.region.y', 'self->pipewire_data.cursor_region.y - source_pos.y')
    portal.write_text(text)
    shutil.copyfile(Path(__file__).with_name("gsr-crop.h"), portal.with_name("mark-shot-crop.h"))
    capture = root / "src/recorder/capture_setup.c"
    text = capture.read_text()
    text = replace(text, '== GSR_DISPLAY_SERVER_X11 && prefer_ximage)', '== GSR_DISPLAY_SERVER_X11 && (prefer_ximage || getenv("MARK_SHOT_X11_UNPRIVILEGED")))')
    capture.write_text(text)
    recorder = root / "src/recorder/recorder.c"
    text = recorder.read_text()
    text = replace(text, '        int ret = avcodec_send_frame(self->video_codec_context, self->video_frame);', '''        int ret = avcodec_send_frame(self->video_codec_context, self->video_frame);
        static bool mark_shot_ready = false;
        if(ret >= 0 && !mark_shot_ready) {
            fprintf(stderr, "MARK_SHOT_RECORDING_READY\\n");
            mark_shot_ready = true;
        }''')
    recorder.write_text(text)


if __name__ == "__main__":
    patch(Path(sys.argv[1]))
