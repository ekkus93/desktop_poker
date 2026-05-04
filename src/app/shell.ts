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
};

export type ParticipantShell = {
  seatIndex: number;
  label: string;
  detail: string;
  kind: "host" | "pending" | "open";
  isLocal: boolean;
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
  };
}

export function createDefaultDisplayName(bootstrap: DesktopBootstrapState) {
  return `Player ${bootstrap.instanceLabel}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isValidPortNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= 65535;
}

export function normalizeHostDraft(
  value: unknown,
  fallbackDraft: HostDraft,
): HostDraft {
  if (!isRecord(value)) {
    return fallbackDraft;
  }

  return {
    tournamentName:
      typeof value.tournamentName === "string" && value.tournamentName.trim()
        ? value.tournamentName
        : fallbackDraft.tournamentName,
    maxPlayers:
      typeof value.maxPlayers === "number" && MAX_PLAYER_OPTIONS.includes(value.maxPlayers)
        ? value.maxPlayers
        : fallbackDraft.maxPlayers,
    startingStack:
      typeof value.startingStack === "number" && STARTING_STACK_OPTIONS.includes(value.startingStack)
        ? value.startingStack
        : fallbackDraft.startingStack,
    blindPresetId:
      typeof value.blindPresetId === "string" && BLIND_PRESETS.some((preset) => preset.id === value.blindPresetId)
        ? value.blindPresetId
        : fallbackDraft.blindPresetId,
    turnTimerSeconds:
      typeof value.turnTimerSeconds === "number" && TURN_TIMER_OPTIONS.includes(value.turnTimerSeconds)
        ? value.turnTimerSeconds
        : fallbackDraft.turnTimerSeconds,
    hostPort: isValidPortNumber(value.hostPort)
      ? value.hostPort
      : fallbackDraft.hostPort,
  };
}

export function buildParticipantShell(
  bootstrap: DesktopBootstrapState,
  hostDraft: HostDraft,
  readySeats: number[],
  recentJoinPayloads: string[],
): ParticipantShell[] {
  const seats: ParticipantShell[] = Array.from({ length: hostDraft.maxPlayers }, (_, index) => ({
    seatIndex: index + 1,
    label: "Open seat",
    detail: "",
    kind: "open" as const,
    isLocal: false,
    ready: false,
  }));

  seats[0] = {
    seatIndex: 1,
    label: "You",
    detail: `Host · ${hostDraft.startingStack} chips`,
    kind: "host",
    isLocal: true,
    ready: readySeats.includes(1),
  };

  if (hostDraft.maxPlayers >= 2) {
    const hasJoinIntent =
      recentJoinPayloads.length > 0 || bootstrap.launchJoinPayload !== null;
    seats[1] = {
      seatIndex: 2,
      label: hasJoinIntent ? "Waiting for player" : "Reserved seat",
      detail: bootstrap.parsedLaunchJoinPayload
        ? "Invite accepted"
        : hasJoinIntent
          ? "Invite received"
          : "Reserved",
      kind: "pending",
      isLocal: false,
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
    return "Checking for a LAN address players on this network can reach...";
  }

  return [
    `Tournament: ${hostDraft.tournamentName}`,
    `Host endpoint: ${resolvedHostIp}:${hostDraft.hostPort}`,
    `Capacity: ${hostDraft.maxPlayers} players · ${hostDraft.startingStack} starting chips`,
    `Blind preset: ${getBlindPreset(hostDraft.blindPresetId).label} (${getBlindPreset(hostDraft.blindPresetId).firstLevel} start)`,
    `Turn timer: ${hostDraft.turnTimerSeconds}s`,
    "Share these table details with the next player so they can join from the Join screen.",
    bootstrap.launchJoinPayload ? "An invite is already attached to this launch." : "No invite is attached to this launch.",
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
