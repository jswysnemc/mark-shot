pub struct PortalMethod {
    pub include_cursor: bool,
}

use crate::backend::wayland::capture::stream;
use log::info;
use std::fs;
use std::os::fd::AsFd;
use std::path::PathBuf;

use crate::backend::CaptureMethod;
use crate::types::{CaptureResult, MonitorFrame, Output, StreamInfo};
use ashpd::desktop::{
    PersistMode, Session,
    screencast::{
        CursorMode, Screencast, SelectSourcesOptions, SourceType as AshpdSourceType, Streams,
    },
};
use async_trait::async_trait;

fn token_path() -> Option<PathBuf> {
    Some(dirs::state_dir()?.join("mark-shot").join("token"))
}

// older builds kept the token in the config dir
fn legacy_token_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("mark-shot").join("token"))
}

fn read_token() -> Option<String> {
    [token_path(), legacy_token_path()]
        .into_iter()
        .flatten()
        .find_map(|path| fs::read_to_string(path).ok())
}

fn write_token(token: &str) {
    let Some(path) = token_path() else { return };
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&path, token));
    match written {
        Ok(()) => {
            if let Some(legacy) = legacy_token_path() {
                let _ = fs::remove_file(legacy);
            }
        }
        Err(e) => info!("Can't save portal token to {}: {}", path.display(), e),
    }
}

fn forget_token() {
    for path in [token_path(), legacy_token_path()].into_iter().flatten() {
        let _ = fs::remove_file(path);
    }
}

async fn start_session(
    proxy: &Screencast,
    token: Option<&str>,
    include_cursor: bool,
) -> Result<(Session<Screencast>, Streams), ashpd::Error> {
    let session = proxy.create_session(Default::default()).await?;

    proxy
        .select_sources(
            &session,
            SelectSourcesOptions::default()
                .set_cursor_mode(if include_cursor {
                    CursorMode::Embedded
                } else {
                    CursorMode::Hidden
                })
                .set_sources(Some(AshpdSourceType::Monitor.into()))
                .set_multiple(true)
                .set_restore_token(token)
                .set_persist_mode(PersistMode::ExplicitlyRevoked),
        )
        .await?;

    let response = proxy
        .start(&session, None, Default::default())
        .await?
        .response()?;

    Ok((session, response))
}

// reconcile portal-reported monitor streams against the wayland output list.
// the portal is a separate, independent source of truth from wayland, so unlike
// present() in the overlay (which trusts its own outputs), here a subset is a
// real, expected situation
fn reconcile_streams(
    streams: Vec<StreamInfo>,
    outputs: &[Output],
) -> Result<Vec<(usize, StreamInfo)>, Box<dyn std::error::Error>> {
    if streams.is_empty() {
        return Err("portal returned no monitor streams".into());
    }

    let mut used = vec![false; outputs.len()];
    let mut matched = Vec::with_capacity(streams.len());

    for stream in streams {
        let pos = stream.position.unwrap_or((0, 0));

        let Some(idx) = outputs
            .iter()
            .enumerate()
            .filter(|(i, _)| !used[*i])
            .find(|(_, o)| o.info.logical_position == Some(pos))
            .map(|(i, _)| i)
        else {
            info!(
                "Skipping portal stream at {:?}, it matches none of the captured outputs (positions: {:?})",
                pos,
                outputs
                    .iter()
                    .map(|o| o.info.logical_position)
                    .collect::<Vec<_>>()
            );
            continue;
        };

        used[idx] = true;
        matched.push((idx, stream));
    }

    if matched.is_empty() {
        return Err(
            "none of the portal's monitor streams matches the monitors being captured".into(),
        );
    }
    matched.sort_by_key(|(idx, _)| *idx);
    Ok(matched)
}

#[async_trait]
impl CaptureMethod for PortalMethod {
    async fn capture_frame(
        &self,
        outputs: &[Output],
    ) -> Result<CaptureResult, Box<dyn std::error::Error>> {
        let proxy = Screencast::new().await?;

        let token = read_token();
        // if portal capture doesn't work, we ask for a new token, since it might be the issue
        let (session, response) =
            match start_session(&proxy, token.as_deref(), self.include_cursor).await {
                Err(ashpd::Error::Portal(ashpd::PortalError::InvalidArgument(msg)))
                    if token.is_some() =>
                {
                    info!("Portal rejected the saved restore token ({msg}), asking again");
                    forget_token();
                    start_session(&proxy, None, self.include_cursor).await?
                }
                res => res?,
            };

        let new_token = response.restore_token().map(str::to_owned);

        let streams_data: Vec<StreamInfo> = response
            .streams()
            .iter()
            .map(|s| StreamInfo {
                node_id: s.pipe_wire_node_id(),
                size: s.size(),
                position: s.position(),
            })
            .collect();

        // reconcile against the wayland outputs BEFORE opening the PipeWire remote,
        // so a mismatch fails fast instead of wasting time on the fd handoff
        let streams_data = reconcile_streams(streams_data, outputs)?;

        let fd = proxy
            .open_pipe_wire_remote(&session, Default::default())
            .await?;

        let mut frames = Vec::new();

        for (output, stream_info) in streams_data {
            let frame = stream::capture_frame(stream_info.node_id, fd.as_fd())
                .map_err(|e| ashpd::Error::Zbus(ashpd::zbus::Error::Failure(e.to_string())))?;

            frames.push(MonitorFrame {
                output,
                pixels: frame.pixels,
                pw_width: frame.width,
                pw_height: frame.height,
                pw_stride: frame.stride,
                info: stream_info,
            });
        }

        // saving only working tokken
        if let Some(new_token) = new_token {
            write_token(&new_token);
        }

        if let Err(e) = session.close().await {
            info!("Can't close portal session: {e}");
        }

        Ok(CaptureResult { frames })
    }
}
