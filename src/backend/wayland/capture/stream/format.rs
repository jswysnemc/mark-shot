//! 【PipeWire】【格式协商】仅接受共享内存可读取的四通道原始视频格式

use pipewire::spa::{self, param::video::VideoFormat, pod::Pod};

/// 【PipeWire】【协商参数】用 buffer 序列化原始视频格式集合，返回借用的协议参数
pub(crate) fn build_format_pod(buffer: &mut Vec<u8>) -> &Pod {
    use spa::pod::serialize::PodSerializer;
    use spa::pod::{ChoiceValue, Object, Property, PropertyFlags, Value};
    use spa::sys::*;
    use spa::utils::{Choice, ChoiceEnum, ChoiceFlags, Id};

    PodSerializer::serialize(
        std::io::Cursor::new(&mut *buffer),
        &Value::Object(Object {
            type_: SPA_TYPE_OBJECT_Format,
            id: SPA_PARAM_EnumFormat,
            properties: vec![
                Property {
                    key: SPA_FORMAT_mediaType,
                    flags: PropertyFlags::empty(),
                    value: Value::Id(Id(SPA_MEDIA_TYPE_video)),
                },
                Property {
                    key: SPA_FORMAT_mediaSubtype,
                    flags: PropertyFlags::empty(),
                    value: Value::Id(Id(SPA_MEDIA_SUBTYPE_raw)),
                },
                Property {
                    // previously it supported only BGRA
                    // added some of the others since cosmic didn't return BGRA
                    key: SPA_FORMAT_VIDEO_format,
                    flags: PropertyFlags::empty(),
                    value: Value::Choice(ChoiceValue::Id(Choice(
                        ChoiceFlags::empty(),
                        ChoiceEnum::Enum {
                            default: Id(VideoFormat::BGRx.as_raw()),
                            alternatives: [
                                VideoFormat::BGRx,
                                VideoFormat::BGRA,
                                VideoFormat::RGBx,
                                VideoFormat::RGBA,
                            ]
                            .iter()
                            .map(|f| Id(f.as_raw()))
                            .collect(),
                        },
                    ))),
                },
            ],
        }),
    )
    .unwrap();

    Pod::from_bytes(buffer).unwrap()
}
