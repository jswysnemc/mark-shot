# Screen recording

**English** | [简体中文](zh-CN/recording.md)

Mark Shot records one monitor, or a region of one monitor, as MP4, MKV or GIF. Capture and
encoding run inside the application; no external recorder is started.

A source build needs the media library first, see
[Installation](installation.md#recording-library). Packages include it.

## Starting a recording

| From | How |
| --- | --- |
| Command line | `mark-shot --record` opens the controller; `mark-shot --record-select` goes straight to region selection |
| Tray | Screen recording → Start recording |
| Editor | `Ctrl+Shift+R` for video, `Ctrl+Shift+G` for GIF |

In the controller, choose the format, audio source, quality, frame rate and countdown, then click
**Start recording**. **Select region** lets you draw the area, which must lie within one monitor.

## Controlling a recording

| Action | Controller or key | Command |
| --- | --- | --- |
| Pause, resume | Pause button, `Ctrl+Shift+P` | `mark-shot --pause-recording`, `mark-shot --resume-recording` |
| Stop and save | Stop button, `Ctrl+Shift+E` | `mark-shot --stop-recording` |
| Query state | | `mark-shot --recording-status` prints JSON |

Paused time is not part of the result. Stopping during the countdown cancels the recording.
Closing the controller stops and saves first. Only one recording can run per configuration and
desktop session; control commands must use the same `--config` as the recording.

## Output

| Format | Video | Audio | Notes |
| --- | --- | --- | --- |
| MP4 | H.264 | AAC | Written while recording |
| MKV | H.264 | AAC | Remuxed without re-encoding after the recording stops |
| GIF | Palette-based | None | Converted after the recording stops |

Files are saved to the `Mark Shot` folder inside the system videos folder unless another folder is
configured. An existing file is never overwritten.

If a recording fails, the controller shows the reason and the working video is kept in a hidden
`.Recording_*.recording` folder next to the output, so that it can be recovered.

## Settings

Available on the **Recording** settings page and in the configuration file:

```toml
[recording]
format = "mp4"          # mp4 | mkv | gif
quality = "balanced"    # balanced | high | efficient
encoder = "auto"        # auto | hardware | software
fps = 30
gif_fps = 12
countdown = 3           # seconds
audio = "off"           # off | system | microphone
cursor = true
directory = ""          # empty uses the videos folder
```

Audio is either the system sound or the default microphone; the two are not mixed. `efficient`
(shown as "Smaller file") lowers the bitrate target, which does not necessarily lower CPU usage.

## Platform behaviour

| Platform | Capture | Encoding |
| --- | --- | --- |
| Wayland with ext-image-copy-capture or wlr-screencopy | In-process protocol session; DMA-BUF when the GPU path is compatible, shared memory otherwise | VAAPI, NVENC, or OpenH264 in software |
| Other Wayland desktops | Desktop portal with a PipeWire stream | VAAPI, NVENC, or OpenH264 |
| X11 | MIT-SHM, or GetImage as a fallback | VAAPI, NVENC, or OpenH264 |
| Windows | Windows Graphics Capture with Direct3D 11, GDI as a fallback | Media Foundation H.264 and AAC |

## Limits

- One monitor or a region of it; several monitors are not combined into one video.
- SDR, 8 bit, 4:2:0. HDR is not supported.
- The recording stops when the monitor layout or the frame size changes; start again afterwards.
- With `encoder = "hardware"`, a failure is reported instead of silently switching to software
  encoding. `auto` chooses an encoder that works.
- On portal-based desktops the recorded source must match the monitor you selected. If it does
  not, region cropping is refused instead of recording the wrong area.
- On Windows an MP4 that was interrupted by a forced termination may not be recoverable.
