import type { DesktopBootstrapState } from "../api/desktop";

export type BlindPreset = {
  id: string;
  label: string;
  summary: string;
  firstLevel: string;
  durationMinutes: number;
};

export type HostDraft = {
  tournamentName: string;
  maxPlayers: number;
  startingStack: number;
  blindPresetId: string;
  turnTimerSeconds: number;
  hostPort: number;
  advancedOpen: boolean;
};

export type ParticipantShell = {
  seatIndex: number;
  label: string;
  detail: string;
  kind: "host" | "pending" | "open";
  ready: boolean;
};

export const BLIND_PRESETS: BlindPreset[] = [
  {
    id: "standard",
    label: "Standard",
    summary: "8 minute levels · steadier local game flow",
    firstLevel: "10 / 20",
    durationMinutes: 8,
  },
  {
    id: "turbo",
    label: "Turbo",
    summary: "5 minute levels · faster test loops",
    firstLevel: "15 / 30",
    durationMinutes: 5,
  },
  {
    id: "deep-stack",
    label: "Deep Stack",
    summary: "10 minute levels · longer post-flop play",
    firstLevel: "10 / 20",
    durationMinutes: 10,
  },
];

export const MAX_PLAYER_OPTIONS = [2, 4, 6, 8, 10];
export const STARTING_STACK_OPTIONS = [1000, 1500, 3000, 5000];
export const TURN_TIMER_OPTIONS = [15, 30, 45, 60];

export function getBlindPreset(blindPresetId: string) {
  return (
    BLIND_PRESETS.find((preset) => preset.id === blindPresetId) ?? BLIND_PRESETS[0]
  );
}

export function createDefaultHostDraft(
  bootstrap: DesktopBootstrapState,
): HostDraft {
  return {
    tournamentName: `Desktop Sit 'n Go ${bootstrap.instanceLabel}`,
    maxPlayers: 6,
    startingStack: 1500,
    blindPresetId: BLIND_PRESETS[0].id,
    turnTimerSeconds: 30,
    hostPort: bootstrap.defaultHostPort,
    advancedOpen: false,
  };
}

export function createDefaultDisplayName(bootstrap: DesktopBootstrapState) {
  return `Player ${bootstrap.instanceLabel}`;
}

export function buildParticipantShell(
  bootstrap: DesktopBootstrapState,
  hostDraft: HostDraft,
  readySeats: number[],
  displayName: string,
  recentJoinPayloads: string[],
): ParticipantShell[] {
  const seats: ParticipantShell[] = Array.from({ length: hostDraft.maxPlayers }, (_, index) => ({
    seatIndex: index + 1,
    label: "Open seat",
    detail: "Waiting for a real LAN participant.",
    kind: "open" as const,
    ready: false,
  }));

  seats[0] = {
    seatIndex: 1,
    label: "You",
    detail: `Host · ${hostDraft.startingStack} chips`,
    kind: "host",
    ready: readySeats.includes(1),
  };

  if (hostDraft.maxPlayers >= 2) {
    const hasJoinIntent =
      recentJoinPayloads.length > 0 || bootstrap.launchJoinPayload !== null;
    seats[1] = {
      seatIndex: 2,
      label: hasJoinIntent ? "Waiting for player" : "Reserved seat",
      detail: bootstrap.parsedLaunchJoinPayload
        ? "Invite accepted on this device"
        : hasJoinIntent
          ? "Invite received"
          : "Ready for the next player",
      kind: "pending",
      ready: readySeats.includes(2),
    };
  }

  return seats;
}

export function buildHostShareText(
  bootstrap: DesktopBootstrapState,
  hostDraft: HostDraft,
  resolvedHostIp: string | null,
  lanError: string | null,
) {
  if (lanError) {
    return [
      "Hosting is blocked until a valid LAN IP is available.",
      lanError,
      "The production app will fail loudly here instead of advertising an unusable endpoint.",
    ].join("\n\n");
  }

  if (!resolvedHostIp) {
    return "Resolving a connectable LAN IP for the host share payload...";
  }

  return [
    `Tournament: ${hostDraft.tournamentName}`,
    `Host endpoint: ${resolvedHostIp}:${hostDraft.hostPort}`,
    `Capacity: ${hostDraft.maxPlayers} players · ${hostDraft.startingStack} starting chips`,
    `Blind preset: ${getBlindPreset(hostDraft.blindPresetId).label} (${getBlindPreset(hostDraft.blindPresetId).firstLevel} start)`,
    `Turn timer: ${hostDraft.turnTimerSeconds}s`,
    "Live pkr1 join payloads appear here after the Rust host listener binds. This shell reserves the share surface without inventing a simulator payload.",
    `Launch payload slot: ${bootstrap.launchJoinPayload ?? "empty"}`,
  ].join("\n");
}

export function storageKey(storageNamespace: string, suffix: string) {
  return `${storageNamespace}:${suffix}`;
}

export function readStoredValue<T>(
  rawValue: string | null,
  fallbackValue: T,
): T {
  if (!rawValue) {
    return fallbackValue;
  }

  try {
    return JSON.parse(rawValue) as T;
  } catch {
    return fallbackValue;
  }
}
