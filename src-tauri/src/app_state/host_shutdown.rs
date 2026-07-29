use super::*;

impl DesktopAppState {
    pub fn runtime_warnings(&self) -> Result<Vec<String>, String> {
        let host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;

        Ok(host_session
            .as_ref()
            .map(|session| runtime_health_warning_messages(&session.host_server.runtime_health()))
            .unwrap_or_default())
    }
}

fn runtime_health_warning_messages(health: &networking::HostRuntimeHealth) -> Vec<String> {
    let mut warnings = Vec::new();

    if health.tick_advance_error_count > 0 || health.state_lock_error_count > 0 {
        warnings.push(
            "The host runtime encountered an internal state error. Stop and restart the table if play is not advancing."
                .to_string(),
        );
    }

    if health.publish_error_count > 0 || health.snapshot_sync_error_count > 0 {
        warnings.push(
            "One or more players missed a table update and may need to reconnect.".to_string(),
        );
    }

    if health.pending_join_limit_rejection_count > 0
        || health.connected_client_limit_rejection_count > 0
    {
        warnings.push(
            "Some connection attempts were rejected because the host reached its safety limit."
                .to_string(),
        );
    }

    warnings
}

impl Drop for DesktopHostSession {
    fn drop(&mut self) {
        self.host_server.request_shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::runtime_health_warning_messages;
    use crate::networking::HostRuntimeHealth;

    #[test]
    fn runtime_health_warnings_are_sanitized_and_actionable() {
        let health = HostRuntimeHealth {
            publish_error_count: 1,
            snapshot_sync_error_count: 2,
            state_lock_error_count: 1,
            pending_join_limit_rejection_count: 1,
            last_error: Some("secret internal transport detail".to_string()),
            ..HostRuntimeHealth::default()
        };

        let warnings = runtime_health_warning_messages(&health);

        assert_eq!(warnings.len(), 3);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("restart the table")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("need to reconnect")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("safety limit")));
        assert!(warnings
            .iter()
            .all(|warning| !warning.contains("secret internal transport detail")));
    }

    #[test]
    fn pending_join_limit_rejection_is_visible_without_raw_error_detail() {
        let health = HostRuntimeHealth {
            pending_join_limit_rejection_count: 1,
            last_error: Some("raw pending-join transport detail".to_string()),
            ..HostRuntimeHealth::default()
        };

        let warnings = runtime_health_warning_messages(&health);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("safety limit"));
        assert!(!warnings[0].contains("raw pending-join transport detail"));
    }

    #[test]
    fn connected_client_limit_rejection_is_visible_without_raw_error_detail() {
        let health = HostRuntimeHealth {
            connected_client_limit_rejection_count: 1,
            last_error: Some("raw connected-client transport detail".to_string()),
            ..HostRuntimeHealth::default()
        };

        let warnings = runtime_health_warning_messages(&health);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("safety limit"));
        assert!(!warnings[0].contains("raw connected-client transport detail"));
    }

    #[test]
    fn host_runtime_health_serialization_keys_are_stable() {
        let value = serde_json::to_value(HostRuntimeHealth::default())
            .expect("host runtime health serializes");
        let mut actual = value
            .as_object()
            .expect("host runtime health serializes as an object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        actual.sort();
        let mut expected = vec![
            "acceptErrorCount",
            "clientRegistryErrorCount",
            "connectedClientLimitRejectionCount",
            "lastError",
            "lastSuccessfulPublishMs",
            "lastSuccessfulTickMs",
            "pendingJoinLimitRejectionCount",
            "publishErrorCount",
            "reconnectMarkErrorCount",
            "snapshotSyncErrorCount",
            "stateLockErrorCount",
            "streamCloneErrorCount",
            "streamTimeoutErrorCount",
            "tickAdvanceErrorCount",
        ];
        expected.sort();

        assert_eq!(actual, expected);
    }

    #[test]
    fn healthy_runtime_has_no_normal_ui_warning() {
        assert!(runtime_health_warning_messages(&HostRuntimeHealth::default()).is_empty());
    }
}
