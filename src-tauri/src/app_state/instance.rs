use std::{env, path::PathBuf};

use dirs::data_local_dir;

use crate::{domain, protocol};

use super::*;

pub(crate) fn detect_instance_profile() -> InstanceProfile {
    derive_instance_profile(
        parse_arg_value(INSTANCE_ID_ARG)
            .or_else(|| env::var(INSTANCE_ID_ENV_VAR).ok())
            .as_deref(),
    )
}

pub(crate) fn detect_launch_join_payload() -> Option<String> {
    parse_arg_value(JOIN_PAYLOAD_ARG)
        .or_else(|| env::var(JOIN_PAYLOAD_ENV_VAR).ok())
        .filter(|value| !value.trim().is_empty())
}

pub(crate) fn parse_launch_join_payload(
    raw_payload: Option<&str>,
) -> (Option<domain::JoinPayload>, Option<String>) {
    match raw_payload {
        Some(payload) => match protocol::decode_join_payload(payload) {
            Ok(join_payload) => (Some(join_payload), None),
            Err(error) => (None, Some(error.to_string())),
        },
        None => (None, None),
    }
}

pub(crate) fn parse_arg_value(flag: &str) -> Option<String> {
    let mut args = env::args().skip(1);

    while let Some(argument) = args.next() {
        if argument == flag {
            return args.next();
        }

        if let Some(value) = argument.strip_prefix(&format!("{flag}=")) {
            return Some(value.to_string());
        }
    }

    None
}

pub(crate) fn derive_instance_profile(raw_instance_label: Option<&str>) -> InstanceProfile {
    let instance_label = raw_instance_label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_INSTANCE_LABEL)
        .to_string();
    let profile_id = sanitize_profile_id(&instance_label);

    InstanceProfile {
        instance_label,
        storage_namespace: format!("desktop-poker:{profile_id}"),
        session_identity: format!("desktop-session:{profile_id}"),
        reconnect_namespace: format!("desktop-reconnect:{profile_id}"),
        profile_directory: detect_profile_directory(&profile_id),
        profile_id,
    }
}

pub(crate) fn sanitize_profile_id(raw_instance_label: &str) -> String {
    let mut profile_id = String::new();
    let mut previous_was_separator = false;

    for character in raw_instance_label.trim().chars() {
        if character.is_ascii_alphanumeric() {
            profile_id.push(character.to_ascii_lowercase());
            previous_was_separator = false;
            continue;
        }

        if !previous_was_separator && !profile_id.is_empty() {
            profile_id.push('-');
            previous_was_separator = true;
        }
    }

    while profile_id.ends_with('-') {
        profile_id.pop();
    }

    if profile_id.is_empty() {
        DEFAULT_INSTANCE_LABEL.to_string()
    } else {
        profile_id
    }
}

pub(crate) fn build_debug_child_instance_id(
    parent_profile_id: &str,
    process_id: u32,
    launch_counter: u32,
) -> String {
    format!("{parent_profile_id}-p{process_id}-client-{launch_counter}")
}

pub(crate) fn build_debug_child_launch_args(
    parent_profile_id: &str,
    process_id: u32,
    launch_counter: u32,
    join_payload: Option<&str>,
) -> (String, Vec<String>) {
    let instance_id = build_debug_child_instance_id(parent_profile_id, process_id, launch_counter);
    let mut args = vec![INSTANCE_ID_ARG.to_string(), instance_id.clone()];

    if let Some(payload) = join_payload
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.push(JOIN_PAYLOAD_ARG.to_string());
        args.push(payload.to_string());
    }

    (instance_id, args)
}

pub(crate) fn detect_profile_directory(instance_id: &str) -> PathBuf {
    data_local_dir()
        .expect("platform data directory is required; cannot fall back to cwd for profile storage")
        .join("desktop-poker")
        .join("profiles")
        .join(instance_id)
}

pub(crate) fn detect_app_data_dir() -> PathBuf {
    data_local_dir()
        .expect("platform data directory is required; cannot fall back to cwd for app-data storage")
        .join("desktop-poker")
}
