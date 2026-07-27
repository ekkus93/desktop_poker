#!/usr/bin/env python3
"""Apply the established TCP-session timeout repair and its regression test."""

from __future__ import annotations

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    content = file_path.read_text(encoding="utf-8")
    occurrences = content.count(old)
    if occurrences != 1:
        raise RuntimeError(
            f"expected exactly one match in {path}, found {occurrences}"
        )
    file_path.write_text(content.replace(old, new, 1), encoding="utf-8")


def append_once(path: str, marker: str, addition: str) -> None:
    file_path = Path(path)
    content = file_path.read_text(encoding="utf-8")
    if marker in content:
        raise RuntimeError(f"repair marker already present in {path}")
    file_path.write_text(content.rstrip() + "\n\n" + addition.rstrip() + "\n", encoding="utf-8")


def main() -> None:
    replace_once(
        "src-tauri/src/networking/runtime/mod.rs",
        "fn validate_production_host_ip(ip_addr: IpAddr) -> Result<(), NetworkingError> {\n",
        "pub(crate) fn clear_established_read_timeout(\n"
        "    stream: &TcpStream,\n"
        "    context: &str,\n"
        ") -> Result<(), NetworkingError> {\n"
        "    stream.set_read_timeout(None).map_err(|error| {\n"
        "        NetworkingError::new(format!(\n"
        "            \"failed to clear {context} read timeout: {error}\"\n"
        "        ))\n"
        "    })\n"
        "}\n\n"
        "fn validate_production_host_ip(ip_addr: IpAddr) -> Result<(), NetworkingError> {\n",
    )

    replace_once(
        "src-tauri/src/networking/runtime/host.rs",
        "                                        if write_json_frame(&mut stream, &snapshot_envelope).is_ok()\n"
        "                                        {\n"
        "                                            let stream_handle =\n",
        "                                        if write_json_frame(&mut stream, &snapshot_envelope).is_ok()\n"
        "                                        {\n"
        "                                            if let Err(error) = clear_established_read_timeout(\n"
        "                                                &stream,\n"
        "                                                \"established host client\",\n"
        "                                            ) {\n"
        "                                                update_health(\n"
        "                                                    &runtime_health_conn2,\n"
        "                                                    |health| {\n"
        "                                                        health.stream_timeout_error_count += 1;\n"
        "                                                        health.record_error(error.to_string());\n"
        "                                                    },\n"
        "                                                );\n"
        "                                                return;\n"
        "                                            }\n"
        "                                            let stream_handle =\n",
    )

    replace_once(
        "src-tauri/src/networking/runtime/client_connect.rs",
        "    write_json_frame(&mut stream, &join_envelope)?;\n"
        "    let snapshot = read_snapshot_response(crypto_provider, &mut stream, join_payload)?;\n\n"
        "    Ok((stream, snapshot))\n",
        "    write_json_frame(&mut stream, &join_envelope)?;\n"
        "    let snapshot = read_snapshot_response(crypto_provider, &mut stream, join_payload)?;\n"
        "    clear_established_read_timeout(&stream, \"established client session\")?;\n\n"
        "    Ok((stream, snapshot))\n",
    )

    replace_once(
        "src-tauri/src/networking/runtime/client_connect.rs",
        "        match read_snapshot_response(crypto_provider, &mut stream, join_payload) {\n"
        "            Ok(snapshot) => return Ok((stream, snapshot)),\n",
        "        match read_snapshot_response(crypto_provider, &mut stream, join_payload) {\n"
        "            Ok(snapshot) => {\n"
        "                clear_established_read_timeout(\n"
        "                    &stream,\n"
        "                    \"reconnected client session\",\n"
        "                )?;\n"
        "                return Ok((stream, snapshot));\n"
        "            }\n",
    )

    append_once(
        "src-tauri/src/networking/runtime/tests/misc.rs",
        "fn established_connections_clear_handshake_read_timeouts()",
        "#[test]\n"
        "fn established_connections_clear_handshake_read_timeouts() {\n"
        "    let provider = DefaultCryptoProvider;\n"
        "    let host = bind_test_host(&provider, \"table-established-timeouts\", 87);\n"
        "    let client = connect_test_client(\n"
        "        &provider,\n"
        "        &host,\n"
        "        \"player-established-timeouts\",\n"
        "        \"Timeout Test\",\n"
        "    );\n"
        "    let _ = expect_snapshot_event(&client);\n\n"
        "    let connected_clients = host.clients.lock().expect(\"host client registry\");\n"
        "    let connected_client = connected_clients\n"
        "        .get(\"player-established-timeouts\")\n"
        "        .expect(\"registered host client\");\n"
        "    let host_stream = connected_client.stream.lock().expect(\"host client stream\");\n"
        "    assert_eq!(\n"
        "        host_stream.read_timeout().expect(\"host read timeout\"),\n"
        "        None,\n"
        "        \"the host must clear the handshake read timeout for an established session\"\n"
        "    );\n"
        "    drop(host_stream);\n"
        "    drop(connected_clients);\n\n"
        "    let command_connection = client\n"
        "        .command_connection\n"
        "        .lock()\n"
        "        .expect(\"client command connection\");\n"
        "    let client_stream = command_connection\n"
        "        .stream\n"
        "        .as_ref()\n"
        "        .expect(\"established client command stream\")\n"
        "        .lock()\n"
        "        .expect(\"client command stream lock\");\n"
        "    assert_eq!(\n"
        "        client_stream.read_timeout().expect(\"client read timeout\"),\n"
        "        None,\n"
        "        \"the client must clear the handshake read timeout for an established session\"\n"
        "    );\n"
        "}\n",
    )


if __name__ == "__main__":
    main()
