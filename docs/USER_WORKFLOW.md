# User Workflow Scenarios

This document captures the main user workflow scenarios for Desktop Poker.

The goal is to describe the workflows from the user's point of view, not from the implementation's point of view. These are planning notes for evaluating navigation, screen flow, copy, and product priorities.

## Principles

- Start from the user's intent, not the internal route structure.
- Prefer a small number of primary flows.
- Treat debug and multi-instance tooling as secondary workflows.
- Keep host, join, play, recover, and review flows distinct.
- Separate common workflows from edge-case or developer-only workflows.

## Workflow groups

The app workflows are easiest to reason about in three groups.

### Core player flows

These are the flows the product should feel built for.

- host a game
- join a game
- get ready
- play at the table
- finish the tournament
- review history
- observe after elimination

These flows should drive:

- the main navigation
- the clearest calls to action
- the shortest and most obvious screen paths
- the least technical copy

### Multi-instance / dev flows

These flows are important for this repository and for QA, but they are not the normal player experience.

- run several desktop instances on one machine
- copy or inject join payloads between instances
- inspect debug state
- launch extra clients from internal tools
- validate local host/join/play behavior quickly

These flows should exist, but they should not shape the main player experience.

### Recovery / error flows

These flows matter when something goes wrong or when the user needs to recover session continuity.

- invalid join payload
- no reachable LAN IP for hosting
- join rejected
- host disconnect
- reconnect attempt
- resync after sequence mismatch
- table unavailable

These flows should be explicit and safe, but they should appear only when needed rather than competing with the core actions.

## Primary user types

### Player-host

A user who creates and runs a local tournament from desktop.

### Player-joiner

A user who joins an existing tournament from desktop using a copied payload.

### Returning player

A user who expects the app to remember local settings, recent payloads, and saved history.

### Eliminated observer

A player who remains in the session after busting and wants to keep watching.

### Developer / tester

A user running multiple desktop instances, validating flows, reproducing bugs, or exercising interop behavior.

## Workflow priority groups

### Tier 1: core player workflows

These are the workflows the product should feel optimized for.

1. Host a local game from desktop.
2. Join a local game from desktop.
3. Play from the main table.
4. Observe after elimination.
5. Recover from disconnect.

### Tier 2: supporting everyday workflows

These are common but secondary to the main host/join/play loop.

1. Reopen the app with saved local state.
2. Review hand history after or between sessions.
3. Launch directly into join flow from a provided payload.

### Tier 3: developer and QA workflows

These matter heavily for this repository, but should not distort the normal user experience.

1. Run multiple local desktop instances.
2. Use debug-only tooling.

## Workflow 1: Host a local game from desktop

### User goal

Create a tournament, share join information, gather players, and start the game.

### Entry points

- Home screen
- Potential future quick-start host action

### Expected happy path

1. Open the app.
2. Choose `Host Tournament`.
3. Enter or confirm tournament settings.
4. Confirm the host machine has a valid LAN address.
5. Copy the join payload or otherwise share it.
6. See players appear in the lobby.
7. Mark the host as ready if needed.
8. Wait for at least one more player to join and become ready.
9. Start the tournament.
10. Transition directly into the table.

### User mental model

The host flow is one continuous setup path:

- configure game
- share join info
- wait for players
- start playing

It should not feel like a tour through internal staging screens.

### Key UI needs

- Clear primary action: `Host Tournament`
- Minimal required settings
- Obvious LAN readiness state
- Obvious join payload sharing affordance
- Clear readiness state for seated players
- One obvious `Start tournament` action

### Common friction risks

- Too many intermediate screens before reaching the table
- Host setup language that describes implementation instead of action
- Disabled sharing controls with unclear reason
- Unclear difference between host setup, lobby, and ready states

## Workflow 2: Join a local game from desktop

### User goal

Paste a payload, validate it, join the tournament, and get into the game.

### Entry points

- Home screen
- Launch with CLI payload
- Launch with environment payload
- Future deep-link entry

### Expected happy path

1. Open the app.
2. Choose `Join Tournament` or arrive there with a payload already attached.
3. Paste the `pkr1_...` payload.
4. Validate the payload.
5. Continue into the lobby.
6. Claim or occupy the expected seat if needed.
7. Mark ready.
8. Wait for host start.
9. Transition into the table.

### User mental model

The join flow should feel like:

- enter invitation
- confirm it looks right
- join the room
- get ready
- play

### Key UI needs

- Single obvious payload entry field
- Immediate validation feedback
- Decoded summary that confirms destination
- Clear next action after validation
- Easy access to recent payloads

### Common friction risks

- Validation succeeds but the next step is vague
- Copy describes internal backend wiring rather than user action
- Join and lobby feel disconnected
- Too many screens before actually reaching play

## Workflow 3: Run multiple local desktop instances

### User goal

Host and join multiple sessions on one machine for testing.

### Entry points

- Terminal launch with different instance ids
- Debug/internal tools

### Expected happy path

1. Launch host instance with explicit instance id.
2. Launch one or more client instances with other ids.
3. Share or inject the join payload.
4. Join from additional instances.
5. Walk through ready, start, play, reconnect, and completion.

### User mental model

Each window is a separate person at the table.

### Key UI needs

- Strong instance labeling
- Distinct local storage and identity
- Easy payload copy and reuse
- No accidental state bleed across windows

### Common friction risks

- Windows look too similar
- Profile identity is hidden
- Debug affordances leak into normal player UX

## Workflow 4: Reopen the app with saved local state

### User goal

Return to the app and keep useful local preferences without manual setup every time.

### Expected happy path

1. Reopen the app.
2. See saved display name already present.
3. See prior host defaults still available.
4. See recent join payloads still available.
5. See saved hand-history summaries still available.
6. Resume from a sensible starting point.

### User mental model

The app should remember convenience state, but not fake live session truth.

### Key UI needs

- Persistent display name
- Persistent host draft
- Persistent recent payloads
- Persistent hand-history summaries
- Persistent window state

### Common friction risks

- Too much saved state noise on home screen
- Persistence details shown as internal storage language instead of useful information

## Workflow 5: Play an active hand from the main table

### User goal

Understand the table state and make the right action when it is their turn.

### Expected happy path

1. Enter the table after start or join.
2. Immediately understand whose turn it is.
3. See board cards, pot, stacks, and local hole cards.
4. Use the action tray when it is the local player's turn.
5. Confirm raises or all-in actions when needed.
6. Follow hand results and continue to the next hand.

### User mental model

The table is the center of the experience. Everything else supports it.

### Key UI needs

- Strong action ownership signal
- Clear community cards and pot size
- Clear local cards
- Clear legal actions
- Minimal confusion between player view and observer view
- History and standings available without overwhelming the main action

### Common friction risks

- Too much explanatory text around the main table
- Table feels like a debug demo instead of a playable screen
- Multiple competing side actions distract from turn ownership

## Workflow 6: Observe after elimination

### User goal

Stay in the session and watch the rest of the tournament after busting.

### Expected happy path

1. Player is eliminated.
2. UI clearly communicates that the player is now observing.
3. Player can still see public state, standings, and history.
4. Action controls are removed or disabled.
5. Tournament continues until completion.

### User mental model

I am still at the table, but I can no longer act.

### Key UI needs

- Clear observer banner
- No confusing inactive action tray
- Clear public-only view
- Strong standings/history visibility

### Common friction risks

- Observer mode feels like a broken player screen
- Hidden-information boundary is not obvious to the user

## Workflow 7: Review hand history after or between sessions

### User goal

Look back at completed hands and outcomes.

### Entry points

- Main table
- Home screen
- Direct navigation to history

### Expected happy path

1. Open hand history.
2. See recent settled hands in clear reverse-chronological order.
3. See pot totals, winners, and eliminations.
4. If live history is unavailable, still see cached summaries.

### User mental model

This is the archive or recap area, not an extension of the active table.

### Key UI needs

- Clear separation from the live table
- Good empty state
- Cached-history fallback with honest messaging

## Workflow 8: Recover from disconnect

### User goal

Reconnect safely without losing track of the tournament.

### Expected happy path

1. Connection drops.
2. UI clearly indicates reconnecting or disconnected state.
3. Reconnect path uses the original identity.
4. The player receives an authoritative snapshot.
5. The player resumes at the correct table state.

### User mental model

I should get back into the same session, not start over.

### Key UI needs

- Clear temporary disconnect state
- Clear reconnect success or failure outcome
- No fake local reconstruction presented as truth

### Common friction risks

- Error states strand the user between screens
- Reconnect language is too technical

## Workflow 9: Launch directly into join flow from a payload

### User goal

Skip manual paste when the payload is already available.

### Entry points

- CLI argument
- Environment variable
- Future deep link

### Expected happy path

1. App launches with payload attached.
2. Join screen is prefilled automatically.
3. User confirms destination details.
4. User proceeds to join.

### User mental model

The invitation should already be loaded for me.

### Key UI needs

- Clear indication that a payload was preloaded
- Easy override if the payload is wrong
- Minimal extra ceremony before join

## Workflow 10: Use debug-only tools during development

### User goal

Inspect state, launch more instances, and reproduce behavior quickly.

### Entry points

- Debug-only route or button

### Expected happy path

1. Open internal tools in debug builds.
2. Inspect snapshot/debug information.
3. Copy payloads.
4. Launch additional instances.
5. Exercise or reproduce targeted flows.

### User mental model

This is a developer workspace, not the main player journey.

### Key UI needs

- Strong separation from production-facing navigation
- High-signal debug information
- Fast multi-instance actions

### Common friction risks

- Debug routes look like normal user routes
- Internal wording leaks into player-facing UI

## Common end-to-end flow clusters

### Host cluster

Home -> Host setup -> Lobby -> Table -> Complete -> History

### Join cluster

Home or launch payload -> Join -> Lobby -> Table -> Complete -> History

### Recovery cluster

Table -> Disconnect state -> Reconnect -> Table

### Observer cluster

Table -> Elimination -> Observer table view -> Complete -> History

### Developer cluster

Debug tools -> Launch more instances -> Host/join/test loops

## Current planning conclusions

### What should feel primary

- Host Tournament
- Join Tournament
- Main Table
- Hand History

### What should feel secondary

- Rules and settings
- Reconnect/error handling
- Tournament complete summary

### What should not dominate the main UX

- Debug tools
- Internal state explanations
- Route-map style navigation
- Extra staging screens that do not represent a distinct user goal

## Next planning step

The next useful planning pass is to map each workflow to:

1. user intent
2. required screens
3. unnecessary screens
4. primary CTA per screen
5. failure states
6. copy requirements

That mapping will make it easier to decide which current screens stay, which collapse together, and which should become internal-only.
