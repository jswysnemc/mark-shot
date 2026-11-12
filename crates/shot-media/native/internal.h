#ifndef MARK_SHOT_MEDIA_INTERNAL_H
#define MARK_SHOT_MEDIA_INTERNAL_H
#include "media.h"
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/audio_fifo.h>
#include <libavutil/hwcontext.h>
#include <libavutil/imgutils.h>
#include <libavutil/opt.h>
#include <libavutil/samplefmt.h>
#include <libswresample/swresample.h>
#include <libswscale/swscale.h>
#include <stdio.h>
#include <string.h>

struct ms_encoder {
    AVFormatContext *output;
    AVCodecContext *video;
    AVCodecContext *audio;
    AVStream *video_stream;
    AVStream *audio_stream;
    AVFrame *pixels;
    AVFrame *hardware;
    AVFrame *sound;
    AVPacket *packet;
    AVBufferRef *device;
    struct SwsContext *scale;
    SwrContext *resample;
    AVAudioFifo *fifo;
    int64_t audio_pts;
    int64_t last_video;
    int header_written;
    int finalized;
    int failed;
    char error[512];
};

int ms_fail(ms_encoder *encoder, const char *stage, int code);
int ms_write(ms_encoder *encoder, AVCodecContext *codec, AVStream *stream, AVFrame *frame);
int ms_video_open(ms_encoder *encoder, const char *device, int width, int height, int fps, int quality, int mode);
int ms_audio_open(ms_encoder *encoder);
int ms_audio_flush(ms_encoder *encoder);
int ms_remux(AVFormatContext *input, AVFormatContext *output, ms_cancel cancel, void *context);
int ms_gif(AVFormatContext *input, AVFormatContext *output, int fps, ms_cancel cancel, void *context);
#endif
