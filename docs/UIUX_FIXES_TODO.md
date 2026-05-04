# UI/UX Fixes TODO

This backlog is for turning Desktop Poker from a debug-friendly desktop harness into a poker app that feels obvious, calm, and worth playing.

Status legend:

- [ ] not started
- [x] done

## 1. Product direction reset

- [x] Write a one-paragraph product bar for the UI
	- [x] Define the app as a poker product first and a dev harness second
	- [x] Define the primary player promise in plain language
	- [x] Define what information should never appear on normal player surfaces

Product bar:

- [x] Desktop Poker is a LAN poker product first: a player should be able to host, join, play, observe, recover, and review a session without needing the implementation explained.
- [x] The player promise is simple: the app should always make the next poker action obvious and keep the table as the center of attention.
- [x] Normal player surfaces must not lead with protocol, payload, profile, namespace, serialization, or instance-management details.

- [x] Define the top UX rule for every screen
	- [x] One primary user goal per screen
	- [x] One obvious primary action per screen
	- [x] No route-structure language on player-facing screens
	- [x] No backend, protocol, storage, or profile jargon on player-facing screens

Top UX rule set:

- [x] Every screen serves one player decision first.
- [x] Every screen exposes one obvious next action before any support content.
- [x] Player-facing titles and leads should describe the table task, not the route or system state shape.
- [x] Backend, protocol, storage, and profile language stays off normal player surfaces.

- [x] Define the UI success bar
	- [x] A new player can host without explanation
	- [x] A new player can join without explanation
	- [x] A seated player can tell whether it is their turn within two seconds
	- [x] A busted player understands they are observing, not broken
	- [x] A disconnected player sees exactly one recovery path

## 2. Remove tool-like shell behavior from the player UI

- [x] Redesign the global app shell
	- [x] Remove persistent sidebar copy that explains the app instead of helping the user act
	- [x] Remove protocol/version badges from the main player shell
	- [x] Remove per-instance technical metadata from the default shell
	- [x] Remove profile-folder visibility from normal player screens
	- [x] Remove permanent "Current focus" helper cards from tournament flow
- [x] Replace the current shell with a simpler product shell
	- [x] Use a lighter top bar or compact navigation instead of a heavy admin-style sidebar
	- [x] Keep primary navigation minimal
	- [x] Keep support destinations visually secondary
	- [x] Keep debug entry completely absent outside debug mode
- [x] Define shell behavior by app phase
	- [x] Pre-tournament shell
	- [x] In-tournament shell
	- [x] Post-tournament shell
	- [x] Error/recovery shell

Shell behavior by phase:

- [x] Pre-tournament shell: keep Host, Join, Rules, and History visible, with Host and Join visually dominant.
- [x] In-tournament shell: keep the table dominant and demote support navigation.
- [x] Post-tournament shell: keep recap and next-step actions visible, with no debug chrome in the main flow.
- [x] Error/recovery shell: show one recovery surface with one clear next action and one safe fallback.

## 3. Fix information hierarchy across the app

- [x] Audit every screen for the first thing the user notices
	- [x] Home
	- [x] Host setup
	- [x] Join
	- [x] Lobby
	- [x] Table
	- [x] History
	- [x] Tournament complete
	- [x] Error/recovery

Hierarchy audit summary:

- [x] Home: the first read is Host versus Join.
- [x] Host setup: the first read is table setup, then sharing.
- [x] Join: the first read is invite entry and review.
- [x] Lobby: the first read is ready state and whether the table can start.
- [x] Table: the first read is the felt surface, acting player, and available actions.
- [x] History: the first read is whether there is useful saved or live history.
- [x] Tournament complete: the first read is the winner and next-step choice.
- [x] Error/recovery: the first read is what happened and what to do next.

- [x] Reduce equal-weight card clutter
	- [x] Identify screens where every card currently competes equally
	- [x] Promote the one primary task area
	- [x] Collapse or demote low-value information blocks
	- [x] Remove decorative sections that do not change decisions
- [x] Create strict visual hierarchy rules
	- [x] One dominant title or task area
	- [x] One dominant CTA
	- [x] Secondary actions grouped together
	- [x] Metadata visually quiet
	- [x] Support content hidden until needed

Hierarchy rules:

- [x] Give each screen one dominant task area and one dominant CTA.
- [x] Group secondary actions together and style them below the primary action.
- [x] Keep badges, counts, and saved-state details visually quiet.
- [x] Hide advanced, debug, and support content until the user asks for it or needs it.

## 4. Rewrite the home screen so it behaves like a product entry point

- [x] Remove non-essential home screen sections from the default view
	- [x] Remove or heavily demote "This device"
	- [x] Remove or heavily demote "Join payload status"
	- [x] Demote saved-state details into a small resume area
- [x] Rebuild home around two actions only
	- [x] Host Tournament
	- [x] Join Tournament
- [x] Design a proper resume section
	- [x] Show recent session recovery only when it matters
	- [x] Show recent payload reuse only when it helps the next step
	- [x] Show hand-history shortcut only when saved history exists
- [x] Tighten copy on the home screen
	- [x] Replace explanation-heavy paragraphs with short action copy
	- [x] Remove implementation-specific terminology
	- [x] Keep labels voice-first and task-first
- [x] Validate the home decision moment
	- [x] User can choose Host vs Join immediately
	- [x] User does not need to scan multiple support cards before acting

## 5. Make host setup feel like one clean setup flow

- [x] Simplify host setup structure
	- [x] Keep the form focused on required setup only
	- [x] Collapse advanced options harder
	- [x] Hide developer-ish hosting details by default
- [x] Improve the share step
	- [x] Make the shareable thing visually obvious
	- [x] Distinguish plain-language share details from raw payload data
	- [x] Avoid showing disabled actions without clear reason
	- [x] If payload copy is unavailable, explain availability briefly and locally
- [x] Improve LAN readiness presentation
	- [x] Replace implementation wording with simple host-readiness language
	- [x] Keep failure copy actionable
	- [x] Remove unnecessary technical explanation from the default state
- [x] Improve flow continuity
	- [x] Make "Continue to lobby" feel like the natural next step
	- [x] Preserve host confidence that setup is complete before continuing

## 6. Make join feel like a normal invitation flow

- [x] Rebuild join around one central action
	- [x] Paste payload
	- [x] Validate payload
	- [x] Confirm destination
	- [x] Continue to lobby
- [x] Remove ambiguity between Validate and Connect
	- [x] Decide whether these are truly separate actions in the UX
	- [x] If they remain separate, explain the distinction with almost no text
	- [x] If they do not need to be separate, collapse them
- [x] Improve payload destination confirmation
	- [x] Show host and table clearly
	- [x] Show just enough metadata to build confidence
	- [x] Hide values that do not help player decision-making
- [x] Make recent payload reuse actually helpful
	- [x] Shorten how recent items are presented
	- [x] Make tapping one feel like a direct reuse action
	- [x] Keep clear/remove actions secondary
- [x] Validate join usability
	- [x] A user knows what to paste
	- [x] A user knows whether the payload is valid
	- [x] A user knows when they can continue

## 7. Make the lobby feel like a table waiting room, not a control panel

- [x] Redesign the lobby around waiting, readiness, and startability
	- [x] Make player readiness instantly scannable
	- [x] Make startability instantly scannable
	- [x] Make the local player’s next step instantly scannable
- [x] Reduce duplicated readiness explanation
	- [x] Remove repetitive helper copy
	- [x] Keep only the one sentence that matters for the current role
- [x] Improve seat map usability
	- [x] Clarify open seats vs occupied seats visually
	- [x] Clarify local seat visually
	- [x] Clarify ready vs waiting visually
- [x] Improve host control area
	- [x] Make Start Tournament the dominant action only when valid
	- [x] Make leave/cancel actions clearly secondary
	- [x] Avoid presenting roster rules as a dense block of copy
- [x] Improve joiner experience in lobby
	- [x] Make ready status feel like a natural acknowledgement step
	- [x] Avoid making every player feel like they are managing the room

## 8. Rebuild the table around actual poker priorities

- [x] Remove non-player mode toggles from the normal table UX
	- [x] Remove manual Player View / Observer View switching from player flow
	- [x] Derive mode automatically from runtime state
	- [x] Keep manual mode switching debug-only if still needed
- [x] Make the top of the table answer only the core questions
	- [x] Is it my turn?
	- [x] What is the board?
	- [x] What is the pot?
	- [x] What action is expected?
- [x] Redesign the action area
	- [x] Put legal actions in one tight, obvious cluster
	- [x] Make the primary action visually dominant
	- [x] Make disabled states self-explanatory
	- [x] Reduce confirmation friction for common actions
	- [x] Keep raise sizing understandable without reading too much
- [x] Redesign seat presentation
	- [x] Make the local seat unmistakable
	- [x] Make the acting seat unmistakable
	- [x] Make eliminated or observing states unmistakable
	- [x] Remove unnecessary seat detail text from the collapsed default state
	- [x] Keep expandable seat detail secondary
- [x] Demote secondary table tools
	- [x] Move standings into a clearly secondary panel or drawer
	- [x] Move public event feed further into the background
	- [x] Keep hand history secondary while a hand is active
	- [x] Remove top-row competition between details/history/standings and the action flow
- [x] Rework table copy
	- [x] Remove text that narrates what the UI already shows
	- [x] Remove terms that sound like QA modes
	- [x] Keep turn-state copy extremely short

## 9. Fix observer and eliminated-player UX

- [x] Make observer state feel intentional
	- [x] Clear banner that the player is now observing
	- [x] No action affordances visible when they cannot act
	- [x] Public table information still feels complete
- [x] Preserve dignity after elimination
	- [x] Avoid visual treatment that looks like an error state
	- [x] Avoid making the screen look disabled or dead
	- [x] Keep standings and recent outcomes available

## 10. Fix hand history so it feels like a supporting product surface

- [x] Improve list readability
	- [x] Make each hand entry scannable in one pass
	- [x] Surface winner, pot, and notable outcome first
	- [x] Demote dense details
- [x] Clarify live vs saved history
	- [x] Tell the user which history they are looking at
	- [x] Avoid storage-centric language
	- [x] Make fallback behavior feel intentional
- [x] Improve empty and sparse states
	- [x] No history yet
	- [x] History exists but table state is unavailable
	- [x] Tournament just started

## 11. Fix tournament completion UX

- [x] Make tournament completion feel final and satisfying
	- [x] Clear winner emphasis
	- [x] Clear final standings
	- [x] Clear next actions
- [x] Reduce placeholder energy
	- [x] Remove anything that reads like an unfinished debug surface
	- [x] Keep summary content short and celebratory
- [x] Improve next-step options
	- [x] Review history
	- [x] Return home
	- [x] Start another game only if that flow is genuinely supported

## 12. Fix error and reconnect UX

- [x] Remove scenario-picker behavior from player flow
	- [x] Keep scenario switching debug-only
	- [x] Ensure default error surfaces always reflect real runtime state
- [x] Simplify all recovery states
	- [x] What happened
	- [x] What it means
	- [x] What to do next
- [x] Reduce duplicate error presentation
	- [x] Avoid showing the same state in both a full screen and dialog preview unless debugging
	- [x] Use one recovery surface per context
- [x] Standardize recovery actions
	- [x] Retry
	- [x] Rejoin
	- [x] Return home
	- [x] Open history
- [x] Validate recovery usability
	- [x] No dead ends
	- [x] No technical diagnostics as the primary message
	- [x] Recovery path clear in under five seconds

## 13. Hide debug and multi-instance plumbing from normal players

- [x] Audit all player-facing surfaces for leaked dev language
	- [x] protocol
	- [x] serialization
	- [x] profile directory
	- [x] namespace
	- [x] session identity
	- [x] reconnect namespace
	- [x] instance id
- [x] Move all debug-heavy controls behind explicit internal tools surfaces
	- [x] mode switches
	- [x] snapshot viewers
	- [x] protocol log viewers
	- [x] client launch helpers
- [x] Separate QA identity from player identity
	- [x] Show display name to players
	- [x] Keep instance labels only where dev workflows need them

## 14. Simplify copy across the whole app

- [x] Create a hard copy rule set
	- [x] Titles under 5 words when possible
	- [x] Helper text one sentence max by default
	- [x] Prefer verbs over explanation
	- [x] Prefer direct instructions over implementation notes

Phase 5 copy rules:

- [x] Keep titles under five words unless poker terminology would become unclear.
- [x] Keep helper text to one sentence by default.
- [x] Lead with the next action, not the implementation.
- [x] Say invite instead of payload on player-facing screens.
- [x] Reserve technical wording for debug-only or internal-review surfaces.

- [x] Audit and shorten every screen lead
	- [x] Home
	- [x] Host
	- [x] Join
	- [x] Lobby
	- [x] Table
	- [x] History
	- [x] Complete
	- [x] Errors
- [x] Remove explanatory duplication
	- [x] Do not say the same thing in title, lead, card header, and hint text
	- [x] Do not explain visible state more than once

## 15. Improve controls, affordances, and interaction density

- [x] Tighten button hierarchy
	- [x] One primary button style used for real primary actions only
	- [x] Ghost and secondary buttons reserved for lower-priority actions
	- [x] Disabled primary buttons must always have a visible reason nearby
- [x] Reduce too many buttons in one row
	- [x] Home
	- [x] Host
	- [x] Table
	- [x] Error states
- [x] Improve form ergonomics
	- [x] Inputs align cleanly
	- [x] Labels stay short
	- [x] Validation states are local and immediate
	- [x] Advanced settings stay collapsed until requested

## 16. Improve visual design so it feels like a game product

- [x] Refine overall look and feel
	- [x] Make the shell feel less like a dashboard
	- [x] Increase table presence relative to navigation chrome
	- [x] Reduce visual weight of secondary panels
- [x] Refine typography
	- [x] Clear scale for page title, section title, body, helper text
	- [x] Reduce amount of small, low-contrast copy used for real decisions
- [x] Refine spacing and grouping
	- [x] More breathing room around primary actions
	- [x] Tighter grouping of related controls
	- [x] Clear separation between primary gameplay and supporting info
- [x] Refine state color usage
	- [x] Success, waiting, warning, error each get one consistent meaning
	- [x] Active-turn emphasis should be visually stronger than generic badge color

## 16A. Create a real art direction and asset system

- [x] Pick an actual visual direction for the product
	- [x] Define the table mood
	- [x] Define whether the game should feel classic casino, modern premium app, home-game friendly, or tournament broadcast inspired
	- [x] Define what visual qualities the app should communicate at first glance
	- [x] Define what visual qualities should be explicitly avoided because they make the app feel like tooling

Chosen direction: classic casino.

Classic casino brief:

- [x] Table mood: rich felt, warm brass or gold accents, dark wood or near-black framing, crisp ivory card faces, restrained glow, and strong center-table focus.
- [x] Product feel target: classic casino first, modern desktop polish second.
- [x] First-glance qualities: premium, legible, confident, table-centered, and unmistakably poker.
- [x] Explicitly avoid: SaaS dashboard chrome, generic blue admin UI, excessive debug badges, flat placeholder cards, and novelty graphics.
- [x] Create a real color palette instead of ad hoc UI colors
	- [x] Pick a primary felt/table color family
	- [x] Pick a shell/background color family
	- [x] Pick one accent color for primary actions
	- [x] Pick one highlight color for active turn state
	- [x] Pick a red family for danger and card suits
	- [x] Pick a neutral scale for text, panels, borders, and subdued metadata
	- [x] Test palette contrast in normal and dim environments
	- [x] Ensure status colors mean one thing consistently across the app

Classic casino palette target:

- [x] Primary felt family: deep green felt with darker emerald shadows and lighter worn-felt highlights.
- [x] Shell/background family: espresso, walnut, charcoal, and near-black framing instead of cold blue panels.
- [x] Primary action accent: antique gold rather than bright product blue.
- [x] Active-turn highlight: warm amber-gold ring or glow reserved for turn ownership.
- [x] Red family: casino-card crimson for hearts, diamonds, danger, and all-in emphasis.
- [x] Neutral scale: ivory, parchment, smoke, and soft graphite for readable text and card surfaces.

Specific palette token set:

```css
:root {
	/* Felt and table */
	--color-felt-950: #0b2418;
	--color-felt-900: #123222;
	--color-felt-800: #1a4630;
	--color-felt-700: #245a3e;

	/* Wood / shell */
	--color-wood-950: #120c09;
	--color-wood-900: #1b130f;
	--color-wood-800: #2a1d17;
	--color-wood-700: #3b2a22;

	/* Gold / brass */
	--color-gold-500: #b78a2f;
	--color-gold-400: #c89b3c;
	--color-gold-300: #ddb45e;
	--color-gold-200: #efd28b;

	/* Ivory / parchment */
	--color-ivory-50: #fbf8f1;
	--color-ivory-100: #f4efe3;
	--color-ivory-200: #e7dcc8;

	/* Graphite neutrals */
	--color-ink-950: #0f1113;
	--color-ink-900: #171a1d;
	--color-ink-800: #24292e;
	--color-ink-700: #394048;
	--color-ink-500: #67707b;
	--color-ink-300: #a8b0b8;

	/* Card red */
	--color-crimson-700: #8f1d22;
	--color-crimson-600: #aa2329;
	--color-crimson-500: #c92f36;

	/* Semantic UI */
	--bg-app: var(--color-wood-950);
	--bg-shell: linear-gradient(180deg, #1b130f 0%, #120c09 100%);
	--bg-panel: rgba(28, 20, 16, 0.9);
	--bg-panel-soft: rgba(38, 27, 21, 0.72);
	--bg-table: radial-gradient(circle at center, #245a3e 0%, #123222 65%, #0b2418 100%);
	--bg-table-rail: linear-gradient(180deg, #3b2a22 0%, #1b130f 100%);

	--text-primary: var(--color-ivory-50);
	--text-secondary: #d8ccb8;
	--text-muted: #a89579;
	--text-on-card: var(--color-ink-950);

	--border-subtle: rgba(231, 220, 200, 0.12);
	--border-strong: rgba(221, 180, 94, 0.35);

	--action-primary-bg: var(--color-gold-400);
	--action-primary-bg-hover: var(--color-gold-300);
	--action-primary-text: #1b130f;
	--action-secondary-bg: rgba(251, 248, 241, 0.06);
	--action-secondary-border: rgba(231, 220, 200, 0.18);
	--action-secondary-text: var(--color-ivory-50);

	--focus-ring: rgba(239, 210, 139, 0.42);
	--turn-highlight: rgba(221, 180, 94, 0.55);

	--status-success: #2d8a57;
	--status-warning: #c2872f;
	--status-danger: #b3363c;
	--status-info: #3f6e8f;

	/* Poker-specific */
	--card-face-bg: linear-gradient(180deg, #fbf8f1 0%, #f4efe3 100%);
	--card-face-border: #d9ccb6;
	--card-face-shadow: rgba(0, 0, 0, 0.24);
	--card-suit-black: #111318;
	--card-suit-red: var(--color-crimson-600);
	--card-back-bg: linear-gradient(180deg, #5b1620 0%, #3d0e15 100%);
	--card-back-border: #d7b66b;
	--card-back-pattern: rgba(255, 244, 214, 0.14);
	--seat-local-border: rgba(221, 180, 94, 0.55);
	--seat-acting-border: rgba(239, 210, 139, 0.72);
	--seat-observer-border: rgba(168, 176, 184, 0.25);
	--seat-eliminated-overlay: rgba(17, 19, 24, 0.45);
}
```

First implementation mapping from current `src/App.css` tokens:

- [x] `--bg-base` -> `--bg-app`
- [x] `--bg-soft` -> `--bg-panel-soft`
- [x] `--bg-panel` -> `--bg-panel`
- [x] `--border-soft` -> `--border-subtle`
- [x] `--text-main` -> `--text-primary`
- [x] `--text-dim` -> `--text-muted`
- [x] `--accent` -> `--action-primary-bg`
- [x] `--accent-soft` -> `--turn-highlight`
- [x] `--success` -> `--status-success`
- [x] `--danger` -> `--status-danger`

Palette implementation rules:

- [x] Felt green should dominate the table surface, not the entire app shell.
- [x] Gold should be reserved for primary actions, active-turn emphasis, and premium focal moments.
- [x] Crimson should mean red suits, danger, or all-in emphasis only.
- [x] Ivory should be used for card faces and key text, not large flat application panels.
- [x] Wood and charcoal should replace the current blue-heavy shell surfaces.
- [x] Generic bright blue should be removed from the main product palette.
- [x] Create an icon system
	- [x] Pick an icon style that matches the product direction
	- [x] Define the minimum icon set needed for the app
	- [x] Choose whether to use a packaged icon library or custom SVG icons
	- [x] Remove leftover scaffold assets that are not part of the product
	- [x] Define where icons are useful and where text-only is better

Classic casino icon direction:

- [x] Icon style: simple engraved-casino style line icons with slightly softened corners, not generic outline SaaS icons.

Chosen icon source:

- [x] Use Lucide as the base icon source.
- [x] License check: Lucide is available under the ISC license, with some inherited Feather icons under MIT.
- [x] Product rule: use icons sparingly for support actions, status cues, and recovery states, not as decoration on every button.

Minimum icon set:

- [x] Host / table setup
- [x] Join / paste / link handoff
- [x] History / recap
- [x] Rules / settings
- [x] Reconnect / retry / warning / error
- [x] Expand / collapse / details
- [x] Internal tools only where explicitly needed

Lucide icon list by screen:

- [x] Home
	- [x] `Flag` for Host Tournament
	- [x] `LogIn` for Join Tournament
	- [x] `History` for hand history shortcuts
	- [x] `Settings` for Rules / Settings
- [x] Host setup
	- [x] `Flag` for host flow heading or host action markers
	- [x] `Copy` for share details
	- [x] `Wifi` for LAN readiness success
	- [x] `TriangleAlert` for LAN readiness failure
	- [x] `ChevronDown` and `ChevronUp` for advanced settings toggle
- [x] Join
	- [x] `Link` for payload or invite context
	- [x] `Clipboard` for pasted payload context
	- [x] `BadgeCheck` for valid payload confirmation
	- [x] `TriangleAlert` for invalid payload state
	- [x] `ArrowRight` for continue to lobby
- [x] Lobby
	- [x] `Users` for participant list or lobby group state
	- [x] `Check` for ready
	- [x] `Clock3` for waiting
	- [x] `Play` for Start Tournament
	- [x] `LogOut` for leave table
- [x] Table
	- [x] `Trophy` only for tournament-complete shortcuts or final-state references
	- [x] `History` for side-panel or history entry points
	- [x] `PanelRight` for details toggle if needed
	- [x] No icons on core betting actions unless user testing proves they improve speed
- [x] History
	- [x] `History` for screen heading or section marker
	- [x] `Trophy` for notable winning summaries only if it improves scanability
- [x] Error and recovery
	- [x] `WifiOff` for connection loss
	- [x] `RefreshCw` for reconnect or retry
	- [x] `TriangleAlert` for failure and blocked states
	- [x] `House` for return-home actions
	- [x] `RotateCcw` for retry or rejoin flows when distinct from reconnect
- [x] Internal tools
	- [x] `Bug` for internal-tools entry
	- [x] `Terminal` for protocol or runtime debugging areas
	- [x] `Activity` for live runtime state or sequence state
	- [x] `Monitor` for multi-instance or window-oriented tooling
	- [x] `Copy` for payload handoff helpers

Icon governance rules:

- [x] One icon meaning should stay stable across the app.
- [x] The same action should not use different icons on different screens.
- [x] Icons should not appear on every card title or every CTA by default.
- [x] Poker actions should prefer readable text over icon-only meaning.
- [x] Icons are for fast recognition, not decoration.
- [x] Internal tools may use a denser icon language than player-facing screens.
- [x] Define a real playing-card asset strategy
	- [x] Decide whether to use custom vector card faces, permissive licensed card art, or high-quality generated vector cards
	- [x] Ensure the chosen card assets are license-safe for the repo
	- [x] Define card face style
	- [x] Define card back style
	- [x] Define compact-card treatment for seat widgets
	- [x] Define board-card treatment for the center table
	- [x] Define hidden-card treatment so it looks intentional, not like a placeholder
	- [x] Replace text-only card rectangles with actual card art assets or polished vector card components

Classic casino card direction:

- [x] Asset strategy target: use a royalty-free traditional vector deck rather than text-only card rectangles.
- [x] Card face style: clean ivory stock, traditional rank and suit layout, subtle border, no novelty illustration.
- [x] Card back style: dark burgundy or deep green back with restrained gold patterning.
- [x] Compact-card treatment: simplified rank-plus-suit rendering that still reads like a real mini card.
- [x] Board-card treatment: larger, high-contrast cards with subtle shadow so the board reads as premium and central.
- [x] Hidden-card treatment: believable card backs, not generic blank blocks.

Chosen card source:

- [x] Use the Tek Eye public-domain SVG playing card deck as the starting source for royalty-free card assets.
- [x] Use the SVG deck as the base and restyle only where needed to fit the classic-casino palette.
- [x] Preserve traditional rank and pip readability over flashy illustration.

Chosen card and back variants:

- [x] Front faces: use the standard traditional Tek Eye front faces as the base.
- [x] Front-face treatment: keep court cards and pip layouts traditional, with only light harmonization of border, shadow, and tone to match the casino palette.
- [x] Card backs: use one simple symmetrical back as the base rather than novelty or scene-heavy backs.
- [x] Card-back treatment: recolor the chosen back to a restrained classic-casino palette, favoring deep burgundy or deep green with gold detailing.
- [x] Compact-card treatment: use simplified mini-card rendering for very small seat-card presentations.
- [x] Board-card treatment: use full card faces for community cards.
- [x] Local hand treatment: use full card faces for the local player's visible hand.
- [x] Hidden-card treatment: use the chosen real card-back art for hidden cards.

Card-variant rules:

- [x] Do not mix multiple card-back designs in the same product.
- [x] Do not use novelty or joke back designs in the player-facing game.
- [x] Do not replace standard ranks and pips with stylized custom symbols that reduce readability.
- [x] Do not force tiny full-detail court art into compact seat widgets when a simplified mini-card is clearer.
- [x] Keep one consistent deck language across board cards, local hand cards, compact cards, and hidden cards.
- [x] Create a real felt and table-surface direction
	- [x] Design the table background/felt treatment
	- [x] Design the board area treatment
	- [x] Design the pot/action area treatment
	- [x] Ensure the table reads as a poker surface before it reads as a web form

Classic casino table direction:

- [x] Table background/felt treatment: oval or softly framed felt table with darker rail edge and subtle texture.
- [x] Board area treatment: center-spot stage for community cards with enough contrast to frame the board as the focal area.
- [x] Pot/action area treatment: dealer-facing action strip with premium chip-count treatment, not generic dashboard badges.

Chosen table-surface rules:

- [x] The table surface is the visual center of the app.
- [x] Navigation chrome must visually step back once a game is active.
- [x] Board cards, pot, acting player, and available actions must sit in the strongest visual band.
- [x] Standings, history, and event feed must read as secondary panels, not co-equal surfaces.
- [x] Define illustration and texture rules
	- [x] Decide whether to use subtle texture, grain, or none
	- [x] Decide whether to use chip, dealer-button, or seat-marker graphics
	- [x] Avoid gimmicky graphics that reduce clarity

Classic casino texture rules:

- [x] Use subtle felt grain and restrained material texture only where it reinforces the table surface.
- [x] Use real dealer-button, seat-marker, and chip-like markers only if they improve scanability.
- [ ] Build a component asset map
	- [ ] Cards
	- [ ] Suits
	- [ ] Dealer marker
	- [ ] Active-turn marker
	- [ ] Ready/waiting state markers
	- [ ] Error/reconnect state icons
	- [ ] History/summary icons if justified
- [x] Create usage rules for all visual assets
	- [x] When icons appear
	- [x] When icons do not appear
	- [x] Minimum sizes
	- [x] Contrast rules
	- [x] Hover/active/disabled behavior

Component visual rules:

- [x] Primary buttons use gold fill and are reserved for the single next-best action.
- [x] Secondary buttons use dark panel treatment with quiet borders.
- [x] Ghost or text-only actions are reserved for low-risk support actions.
- [x] Active-turn state gets the strongest highlight in the interface.
- [x] Local player state gets a premium but quieter border treatment than active-turn state.
- [x] Eliminated and observer states must remain legible but visually demoted.
- [x] Error state uses crimson plus one clear recovery CTA.
- [x] Status badges stay short and never become the primary information carrier.
- [x] Icons appear for support actions, state cues, and recovery surfaces, not as decoration on every control.

Screen-level mockup brief stance:

- [x] User workflows already define the behavioral flow.
- [x] Screen-level mockup briefs should be derived from those workflows, not reinvent them.
- [x] Remaining work is to convert the workflow decisions into concrete visual hierarchy briefs for Home, Join, Lobby, and Table.
- [x] This is not a second workflow-discovery exercise; it is the visual-priority layer that sits on top of the existing workflow decisions.
- [x] Validate the art direction against the product goal
	- [x] Does the app look like a poker game before the user reads any copy?
	- [x] Do the cards look real enough to support actual play?
	- [x] Does the table feel like the center of the product?
	- [x] Does the shell stay secondary to gameplay?
	- [x] Do the assets improve clarity instead of just decorating the UI?

## 17. Improve responsive and narrow-window behavior

- [x] Audit pre-tournament screens at common laptop sizes
	- [x] 1280px wide
	- [x] 1024px wide
	- [x] narrow desktop window
- [x] Audit table usability in narrower windows
	- [x] Ensure action area remains visible without hunting
	- [x] Ensure secondary panels do not dominate
	- [x] Ensure seat cards remain legible
- [x] Define what collapses first on smaller widths
	- [x] support navigation
	- [x] low-value metadata
	- [x] side panels
	- [x] explanatory text

Responsive collapse order:

- [x] At 1280px and below, padding tightens and buttons stack before core task areas collapse.
- [x] At 920px and below, action rows collapse to one column and the action tray becomes a two-column grid.
- [x] At 680px and below, the shell becomes a single-column layout, support grids stack vertically, and the table keeps the board readable with simplified card grids.
- [x] Support navigation, low-value metadata, side panels, and explanatory copy collapse before core table actions.

## 18. Run a full screen-by-screen usability review

- [x] Home review
	- [x] Can a new user choose a path immediately?
	- [x] Is anything noisy or irrelevant?
- [x] Host review
	- [x] Is setup confidence high?
	- [x] Is sharing obvious?
- [x] Join review
	- [x] Is invitation entry obvious?
	- [x] Is next-step confidence high?
- [x] Lobby review
	- [x] Is readiness obvious?
	- [x] Is startability obvious?
- [x] Table review
	- [x] Is turn ownership obvious?
	- [x] Are actions obvious?
	- [x] Are supporting details subordinate?
- [x] History review
	- [x] Is scanability good?
	- [x] Is empty state acceptable?
- [x] Completion review
	- [x] Does it feel complete?
- [x] Error review
	- [x] Does every state have a clear recovery path?

Review notes:

- [x] Home now reads as a choice screen instead of a diagnostics screen.
- [x] Host setup reads as setup, LAN readiness, then sharing.
- [x] Join now reads as paste, review, continue.
- [x] Lobby makes ready state and startability visible before seat detail scanning.
- [x] Table keeps turn ownership and the action tray above support detail panels.
- [x] History and completion screens now read as recap surfaces instead of debug lists.
- [x] Error states resolve to one recovery path plus one safe fallback.

## 19. Add workflow-based UI tests for the fixes

- [x] Home flow tests
	- [x] Host and Join dominate the screen
	- [x] Debug/internal items absent from normal mode
- [x] Host flow tests
	- [x] Required fields only by default
	- [x] Advanced settings remain collapsed until opened
	- [x] Share step and lobby transition are obvious
- [x] Join flow tests
	- [x] Payload entry is primary
	- [x] Continue action appears only when appropriate
- [x] Lobby flow tests
	- [x] Ready and waiting states are easy to distinguish
	- [x] Start button hierarchy behaves correctly
- [x] Table flow tests
	- [x] Only runtime-appropriate mode is shown in normal UX
	- [x] Action area remains primary
	- [x] Secondary panels are hidden by default
- [x] Error flow tests
	- [x] Debug scenario controls absent from normal mode
	- [x] Recovery CTA is always present

## 20. Validate with real human-centered acceptance criteria

- [x] Run a first-impression review with no code/context explanation
	- [x] Can a person tell what this app is for in five seconds?
	- [x] Can a person tell how to start a game in five seconds?
	- [x] Can a person tell how to join a game in five seconds?
- [x] Run a table-state comprehension review
	- [x] Can a person tell whose turn it is in two seconds?
	- [x] Can a person tell whether they can act in two seconds?
	- [x] Can a person tell whether they are eliminated or observing in two seconds?
- [x] Run a frustration audit
	- [x] Identify every place where the UI feels like a tool instead of a game
	- [x] Identify every place where the UI makes the player read too much
	- [x] Identify every place where the UI exposes state that only QA needs

Acceptance review outcome:

- [x] First impression: the app now reads as a poker client with clear Host and Join entry points.
- [x] Table comprehension: acting state, observer state, and available actions are now visually distinct.
- [x] Frustration audit: the remaining QA-heavy details are confined to explicit debug surfaces and tests.

## 21. Execution order

- [x] Phase 1: Strip dev leakage from player UI
	- [x] Remove shell metadata
	- [x] Remove player-facing mode toggles
	- [x] Move debug-only controls out of normal screens
- [x] Phase 2: Rebuild Home, Host, and Join around single-task flows
	- [x] Home
	- [x] Host
	- [x] Join
- [x] Phase 3: Rebuild Lobby and Table hierarchy
	- [x] Lobby
	- [x] Table
	- [x] Observer state
- [x] Phase 4: Rebuild support surfaces
	- [x] History
	- [x] Complete
	- [x] Errors
- [x] Phase 5: Polish visual system and copy
	- [x] Copy pass
	- [x] spacing pass
	- [x] button hierarchy pass
	- [x] responsive pass
- [x] Phase 6: Validate and harden
	- [x] UI tests
	- [x] manual UX review
	- [x] final cleanup

## 22. Definition of done

- [x] The app no longer feels like an internal dashboard during normal play
- [x] The app no longer requires reading long helper text to take the next step
- [x] The home screen feels like a choice, not a diagnostic panel
- [x] The lobby feels like a waiting room, not a control console
- [x] The table feels like a poker table, not a data viewer
- [x] Debug functionality remains available without leaking into player UX
- [x] A normal human can host, join, play, observe, recover, and finish a session without needing the project explained to them first
