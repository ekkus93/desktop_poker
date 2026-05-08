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

export type StoredValueReadResult<T> = {
  value: T;
  hadParseError: boolean;
};

export const BLIND_PRESETS: BlindPreset[] = [
  {
    id: "normal",
    label: "Normal",
    summary: "5 minute levels · balanced Sit 'n Go pace",
    firstLevel: "10 / 20",
    durationMinutes: 5,
  },
  {
    id: "fast",
    label: "Fast",
    summary: "3 minute levels · quickest local loop",
    firstLevel: "10 / 20",
    durationMinutes: 3,
  },
  {
    id: "slow",
    label: "Slow",
    summary: "8 minute levels · longest local structure",
    firstLevel: "10 / 20",
    durationMinutes: 8,
  },
];

export const MAX_PLAYER_OPTIONS = [2, 3, 4, 5, 6, 7, 8, 9, 10];
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

  const legacyBlindPresetId =
    typeof value.blindPresetId === "string"
      ? {
          standard: "normal",
          turbo: "fast",
          "deep-stack": "slow",
        }[value.blindPresetId]
      : undefined;

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
      typeof value.blindPresetId === "string" &&
      BLIND_PRESETS.some((preset) => preset.id === value.blindPresetId)
        ? value.blindPresetId
        : legacyBlindPresetId && BLIND_PRESETS.some((preset) => preset.id === legacyBlindPresetId)
          ? legacyBlindPresetId
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
    "Share these host details with the next player.",
    bootstrap.launchJoinPayload
      ? "This launch already has a compact pkr1_ invite attached."
      : "This text is not a compact pkr1_ invite.",
  ].join("\n");
}

export function storageKey(storageNamespace: string, suffix: string) {
  return `${storageNamespace}:${suffix}`;
}

export function readStoredValue<T>(
  rawValue: string | null,
  fallbackValue: T,
): T {
  return readStoredValueWithStatus(rawValue, fallbackValue).value;
}

export function readStoredValueWithStatus<T>(
  rawValue: string | null,
  fallbackValue: T,
): StoredValueReadResult<T> {
  if (!rawValue) {
    return { value: fallbackValue, hadParseError: false };
  }

  try {
    return {
      value: JSON.parse(rawValue) as T,
      hadParseError: false,
    };
  } catch {
    return { value: fallbackValue, hadParseError: true };
  }
}
