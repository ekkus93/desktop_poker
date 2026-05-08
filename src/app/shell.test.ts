import { describe, expect, it } from "vitest";
import { createBootstrap } from "../test/fixtures";
import {
  BLIND_PRESETS,
  buildHostShareText,
  createDefaultDisplayName,
  createDefaultHostDraft,
  getBlindPreset,
  normalizeHostDraft,
  readStoredValue,
  storageKey,
} from "./shell";

describe("shell helpers", () => {
  describe("getBlindPreset", () => {
    it("returns the matching preset for each known id", () => {
      for (const preset of BLIND_PRESETS) {
        expect(getBlindPreset(preset.id)).toEqual(preset);
      }
    });

    it("falls back to the first preset for an unknown id", () => {
      expect(getBlindPreset("unknown-preset")).toEqual(BLIND_PRESETS[0]);
    });
  });

  describe("createDefaultHostDraft", () => {
    it("derives the default draft from bootstrap values and product defaults", () => {
      const bootstrap = createBootstrap({
        instanceLabel: "Host A",
        defaultHostPort: 49000,
      });

      expect(createDefaultHostDraft(bootstrap)).toEqual({
        tournamentName: "Desktop Sit 'n Go Host A",
        maxPlayers: 6,
        startingStack: 1500,
        blindPresetId: BLIND_PRESETS[0].id,
        turnTimerSeconds: 30,
        hostPort: 49000,
      });
    });
  });

  describe("createDefaultDisplayName", () => {
    it("derives the default player label from the bootstrap instance label", () => {
      expect(
        createDefaultDisplayName(createBootstrap({ instanceLabel: "Client 7" })),
      ).toBe("Player Client 7");
    });

    it("preserves unusual instance labels in the expected format", () => {
      expect(
        createDefaultDisplayName(createBootstrap({ instanceLabel: "north-table/B" })),
      ).toBe("Player north-table/B");
    });
  });

  describe("normalizeHostDraft", () => {
    const fallbackDraft = createDefaultHostDraft(
      createBootstrap({ instanceLabel: "Fallback", defaultHostPort: 43818 }),
    );

    it("preserves a fully valid stored draft", () => {
      const validDraft = {
        tournamentName: "Friday Turbo",
        maxPlayers: 10,
        startingStack: 5000,
        blindPresetId: "slow",
        turnTimerSeconds: 60,
        hostPort: 43900,
      };

      expect(normalizeHostDraft(validDraft, fallbackDraft)).toEqual(validDraft);
    });

    it("falls back completely for non-object input", () => {
      expect(normalizeHostDraft(null, fallbackDraft)).toEqual(fallbackDraft);
      expect(normalizeHostDraft("not-an-object", fallbackDraft)).toEqual(
        fallbackDraft,
      );
    });

    it("falls back individual invalid fields while preserving valid ones", () => {
      expect(
        normalizeHostDraft(
          {
            tournamentName: "   ",
            maxPlayers: 99,
            startingStack: 1500,
            blindPresetId: "invalid",
            turnTimerSeconds: 45,
            hostPort: 0,
          },
          fallbackDraft,
        ),
      ).toEqual({
        tournamentName: fallbackDraft.tournamentName,
        maxPlayers: fallbackDraft.maxPlayers,
        startingStack: 1500,
        blindPresetId: fallbackDraft.blindPresetId,
        turnTimerSeconds: 45,
        hostPort: fallbackDraft.hostPort,
      });
    });

    it("maps legacy stored blind preset ids onto the supported production presets", () => {
      expect(
        normalizeHostDraft(
          {
            ...fallbackDraft,
            blindPresetId: "standard",
          },
          fallbackDraft,
        ).blindPresetId,
      ).toBe("normal");
      expect(
        normalizeHostDraft(
          {
            ...fallbackDraft,
            blindPresetId: "turbo",
          },
          fallbackDraft,
        ).blindPresetId,
      ).toBe("fast");
      expect(
        normalizeHostDraft(
          {
            ...fallbackDraft,
            blindPresetId: "deep-stack",
          },
          fallbackDraft,
        ).blindPresetId,
      ).toBe("slow");
    });

    it("rejects invalid port numbers", () => {
      for (const hostPort of [0, -1, 70000, 43818.5]) {
        expect(
          normalizeHostDraft(
            {
              ...fallbackDraft,
              hostPort,
            },
            fallbackDraft,
          ).hostPort,
        ).toBe(fallbackDraft.hostPort);
      }
    });
  });

  describe("buildHostShareText", () => {
    const bootstrap = createBootstrap();
    const hostDraft = createDefaultHostDraft(bootstrap);

    it("returns the explicit LAN failure text when hosting is blocked", () => {
      const text = buildHostShareText(
        bootstrap,
        hostDraft,
        null,
        "No private IPv4 interface found.",
      );

      expect(text).toContain(
        "Hosting is blocked until a valid LAN IP is available.",
      );
      expect(text).toContain("No private IPv4 interface found.");
    });

    it("returns a loading status while the host LAN address is unresolved", () => {
      expect(buildHostShareText(bootstrap, hostDraft, null, null)).toBe(
        "Checking for a LAN address players on this network can reach...",
      );
    });

    it("builds the happy-path share text with current draft values", () => {
      const text = buildHostShareText(
        bootstrap,
        {
          ...hostDraft,
          tournamentName: "Kitchen Table",
          maxPlayers: 8,
          startingStack: 5000,
          blindPresetId: "fast",
          turnTimerSeconds: 45,
          hostPort: 49001,
        },
        "192.168.1.42",
        null,
      );

      expect(text).toContain("Tournament: Kitchen Table");
      expect(text).toContain("Host endpoint: 192.168.1.42:49001");
      expect(text).toContain("Capacity: 8 players · 5000 starting chips");
      expect(text).toContain("Blind preset: Fast (10 / 20 start)");
      expect(text).toContain("Turn timer: 45s");
      expect(text).toContain("This text is not a compact pkr1_ invite.");
    });

    it("notes when an invite is already attached to this launch", () => {
      expect(
        buildHostShareText(
          createBootstrap({ launchJoinPayload: "pkr1_join" }),
          hostDraft,
          "192.168.1.42",
          null,
        ),
      ).toContain("This launch already has a compact pkr1_ invite attached.");
    });
  });

  describe("storage helpers", () => {
    it("builds namespaced storage keys", () => {
      expect(storageKey("desktop-poker:test", "display-name")).toBe(
        "desktop-poker:test:display-name",
      );
      expect(storageKey("desktop-poker:test", "")).toBe("desktop-poker:test:");
    });

    it("reads stored values with fallback handling", () => {
      expect(readStoredValue<string>(null, "fallback")).toBe("fallback");
      expect(readStoredValue("{\"name\":\"Alice\"}", { name: "fallback" })).toEqual(
        { name: "Alice" },
      );
      expect(readStoredValue("not json", ["fallback"])).toEqual(["fallback"]);
      expect(readStoredValue<number>("7", 1)).toBe(7);
      expect(readStoredValue<string[]>("[\"a\",\"b\"]", [])).toEqual([
        "a",
        "b",
      ]);
    });
  });
});
