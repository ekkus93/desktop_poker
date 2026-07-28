use std::{
    io::Write,
    net::{Shutdown, TcpStream},
    thread,
    time::Duration,
};

use crate::{
    crypto::DefaultCryptoProvider,
    networking::{framing::MAX_FRAME_PAYLOAD_BYTES, ClientRuntimeEvent},
};

use super::support::*;

fn send_raw_peer_frame(host: &super::super::HostServer, bytes: &[u8]) {
    let mut stream = TcpStream::connect(host.listener_addr()).expect("bad peer connects");
    stream.write_all(bytes).expect("bad peer writes test bytes");
    let _ = stream.shutdown(Shutdown::Both);
}

#[test]
fn host_accept_loop_survives_oversized_truncated_and_malformed_join_frames() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-host-abuse", 103);

    let oversized_length = u32::try_from(MAX_FRAME_PAYLOAD_BYTES + 1)
        .expect("configured maximum fits in a frame prefix");
    send_raw_peer_frame(&host, &oversized_length.to_be_bytes());

    let mut truncated_body = 128_u32.to_be_bytes().to_vec();
    truncated_body.extend_from_slice(br#"{"messageType":"JOIN_TOURNAMENT_REQUEST"}"#);
    send_raw_peer_frame(&host, &truncated_body);

    let malformed_json = b"{not-json}";
    let mut malformed_frame = (malformed_json.len() as u32).to_be_bytes().to_vec();
    malformed_frame.extend_from_slice(malformed_json);
    send_raw_peer_frame(&host, &malformed_frame);

    thread::sleep(Duration::from_millis(100));

    let client = connect_test_client(&provider, &host, "player-after-bad-peers", "Healthy Client");
    assert!(matches!(
        client
            .next_event(Duration::from_secs(2))
            .expect("healthy client still receives initial snapshot"),
        ClientRuntimeEvent::Snapshot(_)
    ));
}
