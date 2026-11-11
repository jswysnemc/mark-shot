"""为固定版本 wf-recorder 增加统一暂停时钟、采集前限帧和有界缓冲。"""

from pathlib import Path
import shutil
import sys


def replace(text: str, old: str, new: str) -> str:
    """以唯一 old 更新源码并返回；上下文不匹配时停止构建。"""
    if text.count(old) != 1:
        raise ValueError(f"Unexpected wf-recorder context: {old[:100]}")
    return text.replace(old, new, 1)


def patch(root: Path) -> None:
    """按 root 修改独立组件，在输入节流且暂停时不将旧帧交付编码。"""
    main = root / "src/main.cpp"
    text = '#include "mark-shot-control.hpp"\n' + main.read_text()
    text = replace(text, 'buffer_pool<wf_buffer, 16>', 'buffer_pool<wf_buffer, 4>')
    text = replace(text, '    while(!exit_main_loop)\n    {\n        // wait for a free buffer', '''    signal(SIGUSR2, mark_shot_toggle);
    int64_t next_capture_us = 0;
    const int64_t capture_interval = 1000000 / std::max(1, params.framerate);
    while(!exit_main_loop)
    {
        // 1. 【录制组件】【采集节流】等待目标帧期限，暂停时停止请求新帧
        while(!exit_main_loop && (mark_shot_paused || mark_shot_now() < next_capture_us))
            std::this_thread::sleep_for(std::chrono::milliseconds(2));
        if(exit_main_loop) break;
        next_capture_us = mark_shot_now() + capture_interval;
        // wait for a free buffer''')
    text = replace(text, '        while(buffers.capture().ready_capture() != true) {', '        while(buffers.capture().ready_capture() != true && !exit_main_loop) {')
    text = replace(text, '        bool do_cont = false;', '''        // 2. 【录制组件】【暂停时间】视频与音频都跳过暂停内容，恢复后的时间轴扣除暂停区间
        drop = drop || mark_shot_paused;
        const int64_t adjusted = int64_t(sync_timestamp) - mark_shot_pause_us.load();
        sync_timestamp = uint64_t(std::max<int64_t>(0, adjusted));
        bool do_cont = false;''')
    text = replace(text, '        frame_writer_mutex.unlock();', '''        static bool mark_shot_ready = false;
        if(!drop && do_cont && !mark_shot_ready) {
            fprintf(stderr, "MARK_SHOT_RECORDING_READY\\n");
            mark_shot_ready = true;
        }
        frame_writer_mutex.unlock();''')
    # 录制线程以主线程为信号所有者；音频线程继承屏蔽掩码
    text = replace(text, '    pthread_sigmask(SIG_BLOCK, &sigset, NULL);', '    sigaddset(&sigset, SIGUSR2);\n    pthread_sigmask(SIG_BLOCK, &sigset, NULL);')
    main.write_text(text)
    writer = root / "src/frame-writer.cpp"
    text = '#include "mark-shot-control.hpp"\n#include "mark-shot-avcompat.hpp"\n' + writer.read_text()
    text = replace(text, 'void FrameWriter::add_audio(const void* buffer)\n{', 'void FrameWriter::add_audio(const void* buffer)\n{\n    if(mark_shot_paused) return;')
    for field, function in [('pix_fmts', 'pixel_formats'), ('ch_layouts', 'channel_layouts'), ('sample_fmts', 'sample_formats')]:
        text = text.replace(f'codec->{field}', f'mark_shot_{function}(codec)')
    text = text.replace('codec -> sample_fmts', 'mark_shot_sample_formats(codec)')
    text = replace(text, '    AVDictionary *dummy = NULL;', '''    AVDictionary *dummy = NULL;
    if(params.muxer == "mp4")
        av_dict_set(&dummy, "movflags", "+frag_keyframe+empty_moov+default_base_moof", 0);''')
    writer.write_text(text)
    shutil.copyfile(Path(__file__).with_name("wf-control.hpp"), root / "src/mark-shot-control.hpp")
    shutil.copyfile(Path(__file__).with_name("wf-avcompat.hpp"), root / "src/mark-shot-avcompat.hpp")


if __name__ == "__main__":
    patch(Path(sys.argv[1]))
