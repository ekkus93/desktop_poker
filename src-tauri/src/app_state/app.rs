use std::{
    env,
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    crypto::{self, ProtocolCryptoProvider},
    interop, networking, protocol,
};

use super::*;

impl DesktopAppState {
    #[must_use]
    pub fn detect() -> Self {
        let instance_profile = detect_instance_profile();
        let debug_tools_enabled = cfg!(debug_assertions);
        let launch_join_payload = detect_launch_join_payload();
        let (parsed_launch_join_payload, launch_join_payload_error) =
            parse_launch_join_payload(launch_join_payload.as_deref());
        let app_data_dir = detect_app_data_dir();
        let loaded_provider = crate::npc::provider_storage::load_provider_config(&app_data_dir);
        let llm_api_key_configured = loaded_provider.as_ref().is_some_and(|c| c.is_usable());
        let llm_provider_type = loaded_provider.as_ref().map(|c| {
            serde_json::to_value(&c.provider)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default()
        });
        let llm_provider = Arc::new(Mutex::new(loaded_provider));

        Self {
            bootstrap: DesktopBootstrapState {
                app_name: "Desktop Poker",
                protocol_version: protocol::PROTOCOL_VERSION,
                default_host_port: networking::DEFAULT_HOST_PORT,
                frontend_stack: "React + TypeScript",
                serialization_strategy: protocol::SERIALIZATION_STRATEGY,
                framing_strategy: networking::FRAMING_STRATEGY,
                join_payload_encoding: interop::JOIN_PAYLOAD_ENCODING,
                runtime_transport: networking::RUNTIME_TRANSPORT,
                crypto_stack: crypto::stack(),
                instance_id: instance_profile.profile_id.clone(),
                instance_label: instance_profile.instance_label,
                storage_namespace: instance_profile.storage_namespace,
                session_identity: instance_profile.session_identity,
                reconnect_namespace: instance_profile.reconnect_namespace,
                profile_directory: instance_profile.profile_directory.display().to_string(),
                launch_join_payload,
                parsed_launch_join_payload,
                launch_join_payload_error,
                debug_tools_enabled,
                llm_api_key_configured,
                llm_provider_type,
                backend_modules: backend_modules(),
                screens: screen_catalog(debug_tools_enabled),
            },
            app_data_dir,
            llm_provider,
            debug_table_runtime: Mutex::new(None),
            host_session: Mutex::new(None),
            client_session: Mutex::new(None),
            launched_instances: Mutex::new(0),
        }
    }

    #[must_use]
    pub fn bootstrap(&self) -> DesktopBootstrapState {
        self.bootstrap.clone()
    }

    #[must_use]
    pub fn screen_catalog(&self) -> Vec<ScreenDescriptor> {
        self.bootstrap.screens.clone()
    }

    pub fn table_view(&self, viewer_mode: TableViewerMode) -> Result<TableViewSnapshot, String> {
        if let Some(table_view) = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .as_ref()
            .map(|session| session.table_view(viewer_mode))
            .transpose()?
        {
            return Ok(table_view);
        }

        if let Some(table_view) = self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .map(|session| session.table_view(viewer_mode))
            .transpose()?
        {
            return Ok(table_view);
        }

        let _ = viewer_mode;
        Err("no active live session is available for the table view".to_string())
    }

    pub fn submit_table_action(
        &self,
        viewer_mode: TableViewerMode,
        action_kind: DesktopTableActionKind,
        raise_to_amount: Option<u32>,
    ) -> Result<TableViewSnapshot, String> {
        if let Some(table_view) = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .as_ref()
            .map(|session| session.submit_table_action(viewer_mode, action_kind, raise_to_amount))
            .transpose()?
        {
            return Ok(table_view);
        }

        if let Some(table_view) = self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .map(|session| session.submit_table_action(viewer_mode, action_kind, raise_to_amount))
            .transpose()?
        {
            return Ok(table_view);
        }

        let _ = (viewer_mode, action_kind, raise_to_amount);
        Err("no active live session is available for table actions".to_string())
    }

    pub fn debug_state(&self, viewer_mode: TableViewerMode) -> Result<DebugInspectorState, String> {
        if !self.bootstrap.debug_tools_enabled {
            return Err("debug tools are unavailable in release builds".to_string());
        }

        let mut debug_table_runtime = self
            .debug_table_runtime
            .lock()
            .map_err(|_| "debug table runtime lock poisoned".to_string())?;
        let mut state = debug_table_runtime
            .get_or_insert_with(|| {
                DebugTableRuntime::new().expect("debug table runtime should initialize")
            })
            .debug_state(viewer_mode)?;

        // Attach live tilt levels from the active host session's NPC runner.
        if let Ok(host_session) = self.host_session.lock() {
            if let Some(session) = host_session.as_ref() {
                if let Some(runner) = session.npc_runner.as_ref() {
                    if let Ok(tilt_map) = runner.tilt_levels.lock() {
                        state.npc_tilt_levels = tilt_map.clone();
                    }
                }
            }
        }

        Ok(state)
    }

    pub fn start_host_session(
        &self,
        request: StartHostSessionRequest,
    ) -> Result<HostSessionStatus, String> {
        self.start_host_session_with_mode(request, networking::HostRuntimeMode::Production)
    }

    pub fn host_session_status(&self) -> Result<Option<HostSessionStatus>, String> {
        self.host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .as_ref()
            .map(DesktopHostSession::status)
            .transpose()
    }

    pub fn stop_host_session(&self) -> Result<(), String> {
        self.host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .take();
        Ok(())
    }

    pub fn host_claim_lobby_seat(
        &self,
        request: ClaimLobbySeatRequest,
    ) -> Result<HostSessionStatus, String> {
        let mut host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;
        let session = host_session
            .as_mut()
            .ok_or_else(|| "no active host session".to_string())?;
        session
            .host_server
            .claim_seat(LOCAL_PLAYER_ID, request.seat_index)
            .map_err(|error| error.to_string())?;
        session.status()
    }

    pub fn host_set_lobby_ready_state(
        &self,
        request: SetLobbyReadyStateRequest,
    ) -> Result<HostSessionStatus, String> {
        let mut host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;
        let session = host_session
            .as_mut()
            .ok_or_else(|| "no active host session".to_string())?;
        session
            .host_server
            .set_ready_state(LOCAL_PLAYER_ID, request.is_ready)
            .map_err(|error| error.to_string())?;
        session.status()
    }
    pub fn host_start_tournament(&self) -> Result<HostSessionStatus, String> {
        let mut host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;
        let session = host_session
            .as_mut()
            .ok_or_else(|| "no active host session".to_string())?;
        session
            .host_server
            .start_tournament()
            .map_err(|error| error.to_string())?;
        session.status()
    }

    pub fn join_host_session(
        &self,
        request: JoinHostSessionRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.join_host_session_with_player_id(
            request,
            format!("player-{}", self.bootstrap.instance_id),
        )
    }

    pub fn client_session_status(&self) -> Result<Option<ClientSessionStatus>, String> {
        self.client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .map(DesktopClientSession::status)
            .map(Ok)
            .transpose()
    }

    pub fn leave_client_session(&self) -> Result<(), String> {
        self.client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .take();
        Ok(())
    }

    pub fn client_claim_lobby_seat(
        &self,
        request: ClaimLobbySeatRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .ok_or_else(|| "no active client session".to_string())?
            .claim_lobby_seat(request)
    }

    pub fn client_set_lobby_ready_state(
        &self,
        request: SetLobbyReadyStateRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .ok_or_else(|| "no active client session".to_string())?
            .set_lobby_ready_state(request)
    }

    pub fn launch_additional_client_instance(
        &self,
        join_payload: Option<String>,
    ) -> Result<String, String> {
        if !cfg!(debug_assertions) {
            return Err("debug launch helper is only available in debug builds".to_string());
        }

        let mut launch_counter = self
            .launched_instances
            .lock()
            .map_err(|_| "launch counter lock poisoned".to_string())?;
        *launch_counter += 1;
        let (instance_id, launch_args) = build_debug_child_launch_args(
            &self.bootstrap.instance_id,
            std::process::id(),
            *launch_counter,
            join_payload.as_deref(),
        );
        let current_executable = env::current_exe().map_err(|error| error.to_string())?;
        let current_directory = env::current_dir().map_err(|error| error.to_string())?;

        let mut command = Command::new(current_executable);
        command.current_dir(current_directory);
        for arg in launch_args {
            command.arg(arg);
        }
        command.spawn().map_err(|error| error.to_string())?;

        Ok(instance_id)
    }

    pub(crate) fn start_host_session_with_mode(
        &self,
        request: StartHostSessionRequest,
        runtime_mode: networking::HostRuntimeMode,
    ) -> Result<HostSessionStatus, String> {
        let host_address = request.host_address.trim();
        let tournament_name = request.tournament_name.trim();
        let display_name = request.display_name.trim();

        if host_address.is_empty() {
            return Err("hostAddress must be non-blank".to_string());
        }

        if self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .is_some()
        {
            return Err("leave the active client session before hosting".to_string());
        }

        if self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .is_some()
        {
            return Err("stop the active host session before starting a new table".to_string());
        }

        if tournament_name.is_empty() {
            return Err("tournamentName must be non-blank".to_string());
        }

        if display_name.is_empty() {
            return Err("displayName must be non-blank".to_string());
        }

        if !(2..=10).contains(&request.max_players) {
            return Err("maxPlayers must be between 2 and 10".to_string());
        }

        if request.starting_stack == 0 {
            return Err("startingStack must be greater than zero".to_string());
        }

        if request.turn_timer_seconds == 0 {
            return Err("turnTimerSeconds must be greater than zero".to_string());
        }

        let provider = crypto::DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let session_epoch = now_epoch_ms().max(1);
        let table_id = format!("table-{}-{session_epoch}", self.bootstrap.instance_id);
        let config = build_host_tournament_config(&request)?;
        let snapshot_state = build_initial_host_state(
            &config,
            display_name,
            &table_id,
            session_epoch,
            &host_signing_keys,
            &host_encryption_keys,
        )?;
        let host_server = networking::HostServer::bind(networking::HostRuntimeConfig {
            bind_addr: format!("0.0.0.0:{}", request.host_port)
                .parse()
                .map_err(|error| format!("invalid host port: {error}"))?,
            advertised_host: host_address.to_string(),
            session_epoch,
            table_id,
            table_name: Some(config.tournament_name.clone()),
            join_token: issue_join_token(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state,
            runtime_mode,
        })
        .map_err(|error| error.to_string())?;

        let mut host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;
        *host_session = Some(DesktopHostSession {
            host_server: Arc::new(host_server),
            config,
            advertised_host: host_address.to_string(),
            npc_runner: None,
        });

        host_session
            .as_ref()
            .expect("host session was just inserted")
            .status()
    }

    fn join_host_session_with_player_id(
        &self,
        request: JoinHostSessionRequest,
        player_id: String,
    ) -> Result<ClientSessionStatus, String> {
        let payload = request.join_payload.trim();
        let display_name = request.display_name.trim();

        if payload.is_empty() {
            return Err("joinPayload must be non-blank".to_string());
        }

        if display_name.is_empty() {
            return Err("displayName must be non-blank".to_string());
        }

        if self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .is_some()
        {
            return Err("stop the active host session before joining another table".to_string());
        }

        if self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .is_some()
        {
            return Err("leave the active client session before joining another table".to_string());
        }

        let join_payload =
            protocol::decode_join_payload(payload).map_err(|error| error.to_string())?;
        let provider = crypto::DefaultCryptoProvider;
        let runtime = networking::ClientRuntime::connect(networking::ClientRuntimeConfig {
            join_payload: payload.to_string(),
            player_id,
            display_name: display_name.to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .map_err(|error| error.to_string())?;

        let latest_snapshot = match runtime.next_event(Duration::from_secs(1)) {
            Ok(networking::ClientRuntimeEvent::Snapshot(snapshot)) => {
                client_snapshot_state_from_event(&snapshot)
            }
            Ok(other) => {
                return Err(format!(
                    "expected an initial snapshot event after join, got {other:?}"
                ));
            }
            Err(error) => return Err(error.to_string()),
        };

        let mut client_session = self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?;
        *client_session = Some(DesktopClientSession {
            runtime,
            join_payload,
            latest_snapshot,
            reconnecting: false,
            last_error: None,
            event_feed: Vec::new(),
        });

        Ok(client_session
            .as_mut()
            .expect("client session was just inserted")
            .status())
    }
}
