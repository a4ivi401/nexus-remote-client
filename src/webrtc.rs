use crate::protocol::{ClientMessage, TurnCredentials, WebRtcSignalData};
use gstreamer::prelude::*;
use gstreamer_webrtc::{WebRTCSDPType, WebRTCSessionDescription};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

fn get_screen_capture_source() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "dxgiscreencapsrc"
    }
    #[cfg(target_os = "macos")]
    {
        "avfvideosrc capture-screen=true"
    }
    #[cfg(target_os = "linux")]
    {
        "ximagesrc use-damage=0"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "videotestsrc is-live=true pattern=ball"
    }
}

pub fn create_host_pipeline(
    turn: &TurnCredentials,
) -> Result<gstreamer::Pipeline, Box<dyn Error>> {
    let cap_src = get_screen_capture_source();

    // Build pipeline string with screen capture and H264 low-latency encoder
    let pipeline_str = format!(
        "{cap_src} ! videoconvert ! videoscale ! video/x-raw,width=1920,height=1080,framerate=30/1 ! x264enc tune=zerolatency bitrate=3000 speed-preset=ultrafast key-int-max=30 ! rtph264pay config-interval=1 pt=96 ! application/x-rtp,media=video,encoding-name=H264,payload=96 ! webrtcbin name=webrtc bundle-policy=max-bundle"
    );

    let pipeline = match gstreamer::parse::launch(&pipeline_str) {
        Ok(p) => p.dynamic_cast::<gstreamer::Pipeline>().map_err(|_| "Failed to cast to Pipeline")?,
        Err(e) => {
            warn!("Failed to launch hardware/screen capture pipeline ({e}), falling back to videotestsrc...");
            let fallback_str = "videotestsrc is-live=true pattern=ball ! videoconvert ! video/x-raw,width=1280,height=720,framerate=30/1 ! x264enc tune=zerolatency bitrate=2000 speed-preset=ultrafast ! rtph264pay config-interval=1 pt=96 ! application/x-rtp,media=video,encoding-name=H264,payload=96 ! webrtcbin name=webrtc bundle-policy=max-bundle";
            gstreamer::parse::launch(fallback_str)?
                .dynamic_cast::<gstreamer::Pipeline>()
                .map_err(|_| "Failed to cast fallback pipeline")?
        }
    };

    let webrtc = pipeline.by_name("webrtc").ok_or("webrtcbin not found")?;

    // Configure STUN / TURN servers
    for url in &turn.urls {
        if url.starts_with("stun:") {
            webrtc.set_property("stun-server", url);
        } else if url.starts_with("turn:") {
            webrtc.set_property("turn-server", url);
        }
    }

    Ok(pipeline)
}

pub fn create_viewer_pipeline(
    turn: &TurnCredentials,
) -> Result<gstreamer::Pipeline, Box<dyn Error>> {
    let pipeline_str = "webrtcbin name=webrtc bundle-policy=max-bundle ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink sync=false";

    let pipeline = gstreamer::parse::launch(pipeline_str)?
        .dynamic_cast::<gstreamer::Pipeline>()
        .map_err(|_| "Failed to cast to Pipeline")?;

    let webrtc = pipeline.by_name("webrtc").ok_or("webrtcbin not found")?;

    for url in &turn.urls {
        if url.starts_with("stun:") {
            webrtc.set_property("stun-server", url);
        } else if url.starts_with("turn:") {
            webrtc.set_property("turn-server", url);
        }
    }

    Ok(pipeline)
}

pub async fn run_host_session(
    target_id: String,
    turn: TurnCredentials,
    to_server_tx: mpsc::Sender<ClientMessage>,
    mut from_peer_rx: mpsc::Receiver<String>,
) -> Result<(), Box<dyn Error>> {
    info!("Starting WebRTC Host session with viewer: {}", target_id);

    let pipeline = create_host_pipeline(&turn)?;
    let webrtc = pipeline.by_name("webrtc").ok_or("webrtcbin not found")?;

    let rt = tokio::runtime::Handle::current();

    // Hook ICE candidates
    let to_server_ice = to_server_tx.clone();
    let target_id_ice = target_id.clone();
    let rt_ice = rt.clone();

    webrtc.connect("on-ice-candidate", false, move |values| {
        let mlineindex = values[1].get::<u32>().unwrap();
        let candidate = values[2].get::<String>().unwrap();

        let sig_data = WebRtcSignalData::Candidate {
            candidate,
            sdp_mline_index: mlineindex,
            sdp_mid: None,
        };

        if let Ok(data_json) = serde_json::to_string(&sig_data) {
            let msg = ClientMessage::Signal {
                target_id: target_id_ice.clone(),
                data: data_json,
            };
            let tx = to_server_ice.clone();
            rt_ice.spawn(async move {
                let _ = tx.send(msg).await;
            });
        }
        None
    });

    pipeline.set_state(gstreamer::State::Playing)?;

    // Handle incoming signals from the Viewer
    let webrtc_clone = webrtc.clone();
    tokio::spawn(async move {
        while let Some(raw_data) = from_peer_rx.recv().await {
            match serde_json::from_str::<WebRtcSignalData>(&raw_data) {
                Ok(WebRtcSignalData::Answer { sdp }) => {
                    info!("Received SDP answer from Viewer");
                    if let Ok(desc) = gstreamer_sdp::SDPMessage::parse_buffer(sdp.as_bytes()) {
                        let webrtc_sdp = WebRTCSessionDescription::new(WebRTCSDPType::Answer, desc);
                        webrtc_clone.emit_by_name::<()>(
                            "set-remote-description",
                            &[&webrtc_sdp, &None::<gstreamer::Promise>],
                        );
                    }
                }
                Ok(WebRtcSignalData::Candidate {
                    candidate,
                    sdp_mline_index,
                    ..
                }) => {
                    info!("Received ICE candidate from Viewer");
                    webrtc_clone.emit_by_name::<()>(
                        "add-ice-candidate",
                        &[&sdp_mline_index, &candidate],
                    );
                }
                Ok(WebRtcSignalData::Offer { .. }) => {
                    warn!("Host received unexpected SDP offer from Viewer");
                }
                Err(e) => {
                    error!("Failed to parse WebRTC signal from peer: {}", e);
                }
            }
        }
    });

    // Generate Offer
    let to_server_offer = to_server_tx.clone();
    let target_id_offer = target_id.clone();
    let webrtc_offer = webrtc.clone();
    let rt_offer = rt.clone();

    let promise = gstreamer::Promise::with_change_func(move |reply| {
        let reply = match reply {
            Ok(Some(reply)) => reply,
            _ => return,
        };

        if let Ok(offer) = reply.get::<WebRTCSessionDescription>("offer") {
            webrtc_offer.emit_by_name::<()>(
                "set-local-description",
                &[&offer, &None::<gstreamer::Promise>],
            );

            let sdp_text = offer.sdp().as_text().unwrap_or_default();
            let sig_data = WebRtcSignalData::Offer { sdp: sdp_text };

            if let Ok(data_json) = serde_json::to_string(&sig_data) {
                let msg = ClientMessage::Signal {
                    target_id: target_id_offer.clone(),
                    data: data_json,
                };
                let tx = to_server_offer.clone();
                rt_offer.spawn(async move {
                    let _ = tx.send(msg).await;
                });
            }
        }
    });

    webrtc.emit_by_name::<()>("create-offer", &[&None::<gstreamer::Structure>, &promise]);

    Ok(())
}

pub async fn run_viewer_session(
    target_id: String,
    turn: TurnCredentials,
    to_server_tx: mpsc::Sender<ClientMessage>,
    mut from_peer_rx: mpsc::Receiver<String>,
) -> Result<(), Box<dyn Error>> {
    info!("Starting WebRTC Viewer session connecting to host: {}", target_id);

    let pipeline = create_viewer_pipeline(&turn)?;
    let webrtc = pipeline.by_name("webrtc").ok_or("webrtcbin not found")?;

    let rt = tokio::runtime::Handle::current();

    // Hook ICE candidates
    let to_server_ice = to_server_tx.clone();
    let target_id_ice = target_id.clone();
    let rt_ice = rt.clone();

    webrtc.connect("on-ice-candidate", false, move |values| {
        let mlineindex = values[1].get::<u32>().unwrap();
        let candidate = values[2].get::<String>().unwrap();

        let sig_data = WebRtcSignalData::Candidate {
            candidate,
            sdp_mline_index: mlineindex,
            sdp_mid: None,
        };

        if let Ok(data_json) = serde_json::to_string(&sig_data) {
            let msg = ClientMessage::Signal {
                target_id: target_id_ice.clone(),
                data: data_json,
            };
            let tx = to_server_ice.clone();
            rt_ice.spawn(async move {
                let _ = tx.send(msg).await;
            });
        }
        None
    });

    pipeline.set_state(gstreamer::State::Playing)?;

    let webrtc_clone = webrtc.clone();
    let to_server_ans = to_server_tx.clone();
    let target_id_ans = target_id.clone();

    tokio::spawn(async move {
        while let Some(raw_data) = from_peer_rx.recv().await {
            match serde_json::from_str::<WebRtcSignalData>(&raw_data) {
                Ok(WebRtcSignalData::Offer { sdp }) => {
                    info!("Received SDP offer from Host, creating answer...");
                    if let Ok(desc) = gstreamer_sdp::SDPMessage::parse_buffer(sdp.as_bytes()) {
                        let webrtc_sdp = WebRTCSDPType::Offer;
                        let desc_wrap = WebRTCSessionDescription::new(webrtc_sdp, desc);
                        webrtc_clone.emit_by_name::<()>(
                            "set-remote-description",
                            &[&desc_wrap, &None::<gstreamer::Promise>],
                        );

                        let webrtc_ans = webrtc_clone.clone();
                        let to_srv = to_server_ans.clone();
                        let tgt_id = target_id_ans.clone();
                        let rt_ans = tokio::runtime::Handle::current();

                        let promise = gstreamer::Promise::with_change_func(move |reply| {
                            let reply = match reply {
                                Ok(Some(reply)) => reply,
                                _ => return,
                            };

                            if let Ok(answer) = reply.get::<WebRTCSessionDescription>("answer") {
                                webrtc_ans.emit_by_name::<()>(
                                    "set-local-description",
                                    &[&answer, &None::<gstreamer::Promise>],
                                );

                                let sdp_text = answer.sdp().as_text().unwrap_or_default();
                                let sig_data = WebRtcSignalData::Answer { sdp: sdp_text };

                                if let Ok(data_json) = serde_json::to_string(&sig_data) {
                                    let msg = ClientMessage::Signal {
                                        target_id: tgt_id.clone(),
                                        data: data_json,
                                    };
                                    let tx = to_srv.clone();
                                    rt_ans.spawn(async move {
                                        let _ = tx.send(msg).await;
                                    });
                                }
                            }
                        });

                        webrtc_clone.emit_by_name::<()>(
                            "create-answer",
                            &[&None::<gstreamer::Structure>, &promise],
                        );
                    }
                }
                Ok(WebRtcSignalData::Candidate {
                    candidate,
                    sdp_mline_index,
                    ..
                }) => {
                    info!("Received ICE candidate from Host");
                    webrtc_clone.emit_by_name::<()>(
                        "add-ice-candidate",
                        &[&sdp_mline_index, &candidate],
                    );
                }
                Ok(WebRtcSignalData::Answer { .. }) => {
                    warn!("Viewer received unexpected SDP answer from Host");
                }
                Err(e) => {
                    error!("Failed to parse WebRTC signal from peer: {}", e);
                }
            }
        }
    });

    Ok(())
}
