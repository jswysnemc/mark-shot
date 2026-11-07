//! 【识别进程】【消息编码】读写有界二进制消息和文字结果

use super::*;

#[derive(Default)]
pub(super) struct Body(Vec<u8>);

impl Body {
    pub(super) fn u8(&mut self, v: u8) {
        self.0.push(v);
    }

    pub(super) fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    pub(super) fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    pub(super) fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    pub(super) fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    pub(super) fn bytes(&mut self, v: &[u8]) {
        self.u32(u32::try_from(v.len()).unwrap_or(u32::MAX));
        self.0.extend_from_slice(v);
    }

    pub(super) fn str(&mut self, v: &str) {
        self.bytes(v.as_bytes());
    }

    pub(super) fn path(&mut self, v: &Path) {
        #[cfg(target_os = "linux")]
        self.bytes(v.as_os_str().as_bytes());
        #[cfg(target_os = "windows")]
        self.str(&v.to_string_lossy());
    }

    pub(super) fn state(&mut self, state: &EngineState) {
        match state {
            EngineState::Idle => self.u8(0),
            EngineState::Loading { model } => {
                self.u8(1);
                self.str(model);
            }
            EngineState::Ready { device, model } => {
                self.u8(2);
                self.str(device);
                self.str(model);
            }
            EngineState::Failed(message) => {
                self.u8(3);
                self.str(message);
            }
            EngineState::NoGpu => self.u8(4),
        }
    }

    pub(super) fn send(self, w: &mut impl Write, tag: u8) -> io::Result<()> {
        let len = 1 + self.0.len();
        if len > MAX_SMALL {
            return Err(invalid("message is too large to send"));
        }
        let mut frame = Vec::with_capacity(4 + len);
        frame.extend_from_slice(&(len as u32).to_le_bytes());
        frame.push(tag);
        frame.extend_from_slice(&self.0);
        w.write_all(&frame)?;
        w.flush()
    }
}

pub(super) struct Cursor<'a> {
    pub(super) buf: &'a [u8],
}

impl<'a> Cursor<'a> {
    pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8], ProtoError> {
        if self.buf.len() < n {
            return Err(ProtoError::Malformed("truncated message"));
        }
        let (head, rest) = self.buf.split_at(n);
        self.buf = rest;
        Ok(head)
    }

    pub(super) fn array<const N: usize>(&mut self) -> Result<[u8; N], ProtoError> {
        Ok(self.take(N)?.try_into().expect("take returned N bytes"))
    }

    pub(super) fn u8(&mut self) -> Result<u8, ProtoError> {
        Ok(self.array::<1>()?[0])
    }

    pub(super) fn u16(&mut self) -> Result<u16, ProtoError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(super) fn u32(&mut self) -> Result<u32, ProtoError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(super) fn u64(&mut self) -> Result<u64, ProtoError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub(super) fn f32(&mut self) -> Result<f32, ProtoError> {
        let v = f32::from_le_bytes(self.array()?);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(ProtoError::Malformed("number"))
        }
    }

    pub(super) fn bytes(&mut self) -> Result<&'a [u8], ProtoError> {
        let len = self.u32()? as usize;
        self.take(len)
    }

    pub(super) fn string(&mut self) -> Result<String, ProtoError> {
        std::str::from_utf8(self.bytes()?)
            .map(str::to_owned)
            .map_err(|_| ProtoError::Malformed("text"))
    }

    pub(super) fn path(&mut self) -> Result<PathBuf, ProtoError> {
        #[cfg(target_os = "linux")]
        {
            Ok(PathBuf::from(OsStr::from_bytes(self.bytes()?)))
        }
        #[cfg(target_os = "windows")]
        {
            Ok(PathBuf::from(self.string()?))
        }
    }

    pub(super) fn state(&mut self) -> Result<EngineState, ProtoError> {
        Ok(match self.u8()? {
            0 => EngineState::Idle,
            1 => EngineState::Loading {
                model: self.string()?,
            },
            2 => EngineState::Ready {
                device: self.string()?,
                model: self.string()?,
            },
            3 => EngineState::Failed(self.string()?),
            4 => EngineState::NoGpu,
            _ => return Err(ProtoError::Malformed("engine state")),
        })
    }

    pub(super) fn lines(&mut self) -> Result<OcrText, ProtoError> {
        let count = self.u32()? as usize;
        if count > MAX_LINES {
            return Err(ProtoError::Malformed("line count"));
        }
        let mut lines = Vec::with_capacity(count.min(self.buf.len() / 24));
        for _ in 0..count {
            let text = self.string()?;
            let (l, t, r, b) = (self.f32()?, self.f32()?, self.f32()?, self.f32()?);
            let bounds = Rect::from_ltrb(l, t, r, b).ok_or(ProtoError::Malformed("line bounds"))?;
            let n = self.u32()? as usize;
            if n > self.buf.len() / 4 {
                return Err(ProtoError::Malformed("truncated message"));
            }
            let char_x = (0..n).map(|_| self.f32()).collect::<Result<Vec<_>, _>>()?;
            lines.push(OcrLine {
                text,
                bounds,
                char_x,
            });
        }
        Ok(OcrText { lines })
    }

    pub(super) fn finish(&self) -> Result<(), ProtoError> {
        if self.buf.is_empty() {
            Ok(())
        } else {
            Err(ProtoError::Malformed("trailing bytes"))
        }
    }
}
