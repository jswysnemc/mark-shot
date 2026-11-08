// The clipboard helper process. Wayland hands the selection to a live process,
// so the overlay re-execs itself here and this one stays alive to serve the
// data until something else takes the clipboard over.

use std::io::{Read, Write};
use std::time::Instant;

use log::{error, info};
use wl_clipboard_rs::copy::{ClipboardType, MimeSource, MimeType, Options, Source};

use super::{PRIMARY_ARG, TEXT_ARG};

/// 【剪贴板】【辅助入口】按 args 选择图片或文本服务，图片逐块接收并存储；无返回值，启动失败以非零状态结束
pub fn cli(args: impl Iterator<Item = String>) {
    let args: Vec<String> = args.collect();
    let text = args.iter().any(|a| a == TEXT_ARG);
    let primary = args.iter().any(|a| a == PRIMARY_ARG);

    if !text {
        if let Err(error) = super::image_service::serve(primary) {
            println!("{}", error.to_string().replace('\n', " "));
            error!("【图片剪贴板】【服务失败】{error}");
            std::process::exit(1);
        }
        return;
    }

    let mut buf = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut buf) {
        println!("can't read clipboard data: {e}");
        std::process::exit(1);
    }

    let mut opts = Options::new();
    opts.foreground(true);
    opts.clipboard(if primary {
        ClipboardType::Primary
    } else {
        ClipboardType::Regular
    });

    let len = buf.len();
    let kind = if text { "text" } else { "image" };
    let target = if primary { "primary" } else { "clipboard" };
    let sources = if text {
        vec![MimeSource {
            source: Source::Bytes(buf.into()),
            mime_type: MimeType::Text,
        }]
    } else {
        vec![
            MimeSource {
                source: Source::Bytes(buf.into()),
                mime_type: MimeType::Specific("image/png".to_string()),
            },
            MimeSource {
                source: Source::Bytes(Vec::new().into()),
                mime_type: MimeType::Specific("x-kde-force-image-copy".to_string()),
            },
        ]
    };

    // the parent reads this line: empty once the selection is ours
    let mut stdout = std::io::stdout();
    match opts.prepare_copy_multi(sources) {
        Ok(copy) => {
            let _ = writeln!(stdout);
            let _ = stdout.flush();
            info!("serving {len} bytes of {kind} to {target}");
            let started = Instant::now();
            match copy.serve() {
                Ok(()) => info!("taken over after {:.1?}", started.elapsed()),
                Err(e) => {
                    error!("stopped serving after {:.1?}: {e:?}", started.elapsed());
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            let _ = writeln!(stdout, "{}", e.to_string().replace('\n', " "));
            std::process::exit(1);
        }
    }
}
