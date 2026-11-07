#[path = "protocol/codec.rs"]
mod codec;
use codec::{Body, Cursor};

// UNIX socket protocol for the OCR daemon.
// Framing structure: u32 LE length (tag + body) | u8 tag | body.
// Numeric fields are little-endian; strings and byte buffers are prefixed with a u32 length.
// The initial `Hello` and `Welcome` handshakes (carrying `proto` and `build` identifiers) must retain their layout:
// they allow cross-version process recognition, triggering out-of-date daemons to step down.

#[cfg(target_os = "linux")]
use std::ffi::OsStr;
use std::fmt;
use std::io::{self, Read, Write};
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use tiny_skia::Rect;

use crate::ocr::models::ModelFiles;
use crate::ocr::{OcrImage, OcrLine, OcrText};

pub const PROTO: u16 = 1;
/// `Recognize` frame payload
///  full desktop screen capture spanning multiple display monitors.
pub const MAX_FRAME: usize = 256 * 1024 * 1024;
/// All remaining frame types.
pub const MAX_SMALL: usize = 16 * 1024 * 1024;
pub const MAX_LINES: usize = 100_000;
const IMAGE_HEAD: usize = 16;

mod tag {
    pub const HELLO: u8 = 1;
    pub const WELCOME: u8 = 2;
    pub const SHUTDOWN: u8 = 3;
    pub const OK: u8 = 4;
    pub const ERROR: u8 = 5;
    pub const LOAD: u8 = 6;
    pub const RECOGNIZE: u8 = 7;
    pub const LINES: u8 = 8;
    pub const STATUS: u8 = 9;
    pub const STATUS_REPLY: u8 = 10;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineState {
    Idle,
    Loading { model: String },
    Ready { device: String, model: String },
    Failed(String),
    NoGpu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Welcome {
    pub proto: u16,
    pub build: String,
    pub state: EngineState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub pid: u32,
    pub uptime_secs: u64,
    pub state: EngineState,
    pub served: u64,
    pub last_ms: u32,
    pub rss_kb: u64,
    pub idle_left_secs: Option<u64>,
}

pub enum Request {
    Hello { proto: u16, build: String },
    Load(ModelFiles),
    Recognize(OcrImage),
    Status,
    Shutdown,
}

/// Recognition request payload
/// image buffer is borrowed so it can be processed locally if daemon RPC fails.
#[derive(Clone, Copy)]
pub enum Outgoing<'a> {
    Hello(&'a str),
    Load(&'a ModelFiles),
    Recognize(&'a OcrImage),
    Status,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    Welcome(Welcome),
    Ok,
    Error(String),
    Lines(OcrText),
    Status(Status),
}

#[derive(Debug)]
pub enum ProtoError {
    Io(io::Error),
    TooLarge(usize),
    Malformed(&'static str),
    UnknownTag(u8),
}

impl fmt::Display for ProtoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtoError::Io(e) => write!(f, "{e}"),
            ProtoError::TooLarge(len) => write!(f, "frame of {len} bytes is too large"),
            ProtoError::Malformed(what) => write!(f, "malformed {what}"),
            ProtoError::UnknownTag(tag) => write!(f, "unknown message {tag}"),
        }
    }
}

impl std::error::Error for ProtoError {}

impl From<io::Error> for ProtoError {
    fn from(e: io::Error) -> Self {
        ProtoError::Io(e)
    }
}

pub fn write_request(w: &mut impl Write, request: Outgoing) -> io::Result<()> {
    let mut body = Body::default();
    match request {
        Outgoing::Hello(build) => {
            body.u16(PROTO);
            body.str(build);
            body.send(w, tag::HELLO)
        }
        Outgoing::Load(files) => {
            body.path(&files.detector);
            body.path(&files.recognizer);
            body.path(&files.dict);
            body.send(w, tag::LOAD)
        }
        Outgoing::Recognize(image) => write_image(w, image),
        Outgoing::Status => body.send(w, tag::STATUS),
        Outgoing::Shutdown => body.send(w, tag::SHUTDOWN),
    }
}

pub fn read_request(r: &mut impl Read) -> Result<Request, ProtoError> {
    let (tag, len) = read_head(r)?;
    if tag == tag::RECOGNIZE {
        return read_image(r, len).map(Request::Recognize);
    }
    let body = read_small(r, len)?;
    let mut c = Cursor { buf: &body };
    let request = match tag {
        tag::HELLO => {
            return Ok(Request::Hello {
                proto: c.u16()?,
                build: c.string()?,
            });
        }
        tag::LOAD => Request::Load(ModelFiles {
            detector: c.path()?,
            recognizer: c.path()?,
            dict: c.path()?,
        }),
        tag::STATUS => Request::Status,
        tag::SHUTDOWN => Request::Shutdown,
        other => return Err(ProtoError::UnknownTag(other)),
    };
    c.finish()?;
    Ok(request)
}

pub fn write_reply(w: &mut impl Write, reply: &Reply) -> io::Result<()> {
    let mut body = Body::default();
    let tag = match reply {
        Reply::Welcome(welcome) => {
            body.u16(welcome.proto);
            body.str(&welcome.build);
            body.state(&welcome.state);
            tag::WELCOME
        }
        Reply::Ok => tag::OK,
        Reply::Error(message) => {
            body.str(message);
            tag::ERROR
        }
        Reply::Lines(text) => {
            if text.lines.len() > MAX_LINES {
                return Err(invalid("too many lines to send"));
            }
            body.u32(text.lines.len() as u32);
            for line in &text.lines {
                body.str(&line.text);
                let b = line.bounds;
                for v in [b.left(), b.top(), b.right(), b.bottom()] {
                    body.f32(v);
                }
                body.u32(line.char_x.len() as u32);
                for &x in &line.char_x {
                    body.f32(x);
                }
            }
            tag::LINES
        }
        Reply::Status(status) => {
            body.u32(status.pid);
            body.u64(status.uptime_secs);
            body.state(&status.state);
            body.u64(status.served);
            body.u32(status.last_ms);
            body.u64(status.rss_kb);
            match status.idle_left_secs {
                Some(secs) => {
                    body.u8(1);
                    body.u64(secs);
                }
                None => body.u8(0),
            }
            tag::STATUS_REPLY
        }
    };
    body.send(w, tag)
}

pub fn read_reply(r: &mut impl Read) -> Result<Reply, ProtoError> {
    let (tag, len) = read_head(r)?;
    let body = read_small(r, len)?;
    let mut c = Cursor { buf: &body };
    let reply = match tag {
        tag::WELCOME => {
            let proto = c.u16()?;
            let build = c.string()?;
            if proto != PROTO {
                // Skip parsing state from mismatched versions. Daemon instance will be replaced regardless.

                return Ok(Reply::Welcome(Welcome {
                    proto,
                    build,
                    state: EngineState::Idle,
                }));
            }
            Reply::Welcome(Welcome {
                proto,
                build,
                state: c.state()?,
            })
        }
        tag::OK => Reply::Ok,
        tag::ERROR => Reply::Error(c.string()?),
        tag::LINES => Reply::Lines(c.lines()?),
        tag::STATUS_REPLY => Reply::Status(Status {
            pid: c.u32()?,
            uptime_secs: c.u64()?,
            state: c.state()?,
            served: c.u64()?,
            last_ms: c.u32()?,
            rss_kb: c.u64()?,
            idle_left_secs: match c.u8()? {
                0 => None,
                1 => Some(c.u64()?),
                _ => return Err(ProtoError::Malformed("status")),
            },
        }),
        other => return Err(ProtoError::UnknownTag(other)),
    };
    c.finish()?;
    Ok(reply)
}

fn write_image(w: &mut impl Write, image: &OcrImage) -> io::Result<()> {
    let pixels = image_len(image.width, image.height)
        .filter(|&n| n == image.rgb.len())
        .ok_or_else(|| invalid("image buffer does not match its size"))?;
    let len = 1 + IMAGE_HEAD + pixels;
    if len > MAX_FRAME {
        return Err(invalid("image is too large to send"));
    }
    let mut head = Vec::with_capacity(5 + IMAGE_HEAD);
    head.extend_from_slice(&(len as u32).to_le_bytes());
    head.push(tag::RECOGNIZE);
    head.extend_from_slice(&image.width.to_le_bytes());
    head.extend_from_slice(&image.height.to_le_bytes());
    head.extend_from_slice(&image.origin.0.to_le_bytes());
    head.extend_from_slice(&image.origin.1.to_le_bytes());
    // pixels written directly, avoiding intermediate buffer copies.
    w.write_all(&head)?;
    w.write_all(&image.rgb)?;
    w.flush()
}

fn read_image(r: &mut impl Read, len: usize) -> Result<OcrImage, ProtoError> {
    if len < IMAGE_HEAD {
        return Err(ProtoError::Malformed("image"));
    }
    let mut head = [0u8; IMAGE_HEAD];
    r.read_exact(&mut head)?;
    let mut c = Cursor { buf: &head };
    let (width, height) = (c.u32()?, c.u32()?);
    let origin = (c.f32()?, c.f32()?);
    let pixels = image_len(width, height)
        .filter(|&n| n == len - IMAGE_HEAD)
        .ok_or(ProtoError::Malformed("image size"))?;
    let mut rgb = vec![0u8; pixels];
    r.read_exact(&mut rgb)?;
    Ok(OcrImage {
        rgb,
        width,
        height,
        origin,
    })
}

/// RGB8 image byte buffer.
/// returns `None` if empty or if dimension size overflows.
fn image_len(width: u32, height: u32) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(3)
        .filter(|&n| n > 0)
}

fn read_head(r: &mut impl Read) -> Result<(u8, usize), ProtoError> {
    let mut head = [0u8; 5];
    r.read_exact(&mut head)?;
    let len = u32::from_le_bytes([head[0], head[1], head[2], head[3]]) as usize;
    if len == 0 {
        return Err(ProtoError::Malformed("frame"));
    }
    if len > MAX_FRAME {
        return Err(ProtoError::TooLarge(len));
    }
    Ok((head[4], len - 1))
}

fn read_small(r: &mut impl Read, len: usize) -> Result<Vec<u8>, ProtoError> {
    if len + 1 > MAX_SMALL {
        return Err(ProtoError::TooLarge(len + 1));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    Ok(body)
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
#[path = "protocol/tests.rs"]
mod tests;
