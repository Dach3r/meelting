# Stabilization backlog

The one list of deferred work needed to go from the functional MVP (the meeting with voice, chat, agents and tools, [roadmap.md](roadmap.md#meeting-mvp-product-phase)) to a robust and complete product. Scattered TODOs in the code are not used: everything deferred lives here, linked from the document it affects.

**Classes:**
- **Critical now:** compromises data, isolation or essential permissions, or prevents using the main flow.
- **Post-MVP:** performance, recovery, more tests, accessibility, compatibility and advanced experience.
- **Future evolution:** new product capabilities.

Each entry says what is missing, why it is deferred, the current risk or limit, its priority and a verifiable criterion to close it.

## Critical now

| # | What is missing | Why deferred | Current risk or limit | Priority | Done when |
|---|---|---|---|---|---|
| S-01 | **Manual check of the microphone in the app** (macOS permission dialog, capture, transcription, sending) | The permission dialog needs a person at the screen | Prepared on 2026-11-20: the start no longer blocks the main thread while the dialog is open, a denied permission (all-zero samples) maps to `microphone_unavailable`, silence to `nothing_heard`, and whisper annotations are stripped (tested in `domain/speech.rs`). Real sound has not been captured yet. Check with the debug bundle (`src-tauri/target/aarch64-apple-darwin/debug/bundle/macos/Meelting.app`, see voice-engine.md), since `tauri dev` asks the permission for the terminal | P0 | A person clicks «Talk» in a room, accepts the permission, says a sentence and sees it sent as their message; also with the permission denied (`tccutil reset Microphone app.meelting.desktop`, then «Don't Allow»), the room shows `microphone_unavailable` |
| S-02 | **Signed bundle with microphone entitlement** (`com.apple.security.device.audio-input` under the hardened runtime) | Signing and notarization are Phase 7 | `Info.plist` has the usage text; a signed, hardened build without the entitlement will be refused microphone access | P0 before the first signed release | A notarized `.dmg` captures audio after the permission dialog |

## Post-MVP

| # | What is missing | Why deferred | Current risk or limit | Priority | Done when |
|---|---|---|---|---|---|
| S-10 | Voice on Windows and Linux (capture, STT build, TTS) | MVP is macOS first; whisper.cpp needs MSVC + LLVM in CI and Linux TTS is weak (voice-engine §11) | Voice controls are hidden outside macOS (`available: false`) | P1 | Voice works on Windows and Ubuntu CI builds and a manual check on each |
| S-11 | Agent tools on Windows and Linux (D56 refuses them) | Real-volume NTFS checks and Linux runs are pending (BR-17) | Tools are refused outside macOS | P1 | BR-17 Windows items closed, manual run on each OS |
| S-12 | Push-to-talk keyboard shortcut (P17) | The key is not decided | Only the button starts and sends a turn | P2 | A documented key works only with the window focused, with a test |
| S-13 | Model download progress and resume | A single 60 MB file downloads in seconds on a normal connection | The dialog only says «Downloading…»; a cut connection restarts from zero | P2 | Progress percentage is shown and an interrupted download resumes or retries clearly |
| S-14 | Releasing the speech model after inactivity | Memory is modest (base model ~60 MB) | The model stays loaded until the app closes | P3 | The model is released after N minutes without voice and reloaded on demand |
| S-15 | Short spoken replies | The agent prompt does not know the reply will be heard | Long answers are cut at 1,200 characters when read aloud (the full text stays in the chat) | P2 | When replies are read aloud the prompt asks for a short spoken summary, with a test on the prompt |
| S-16 | Per-agent voice choice (D8, `agent_voices`) | Needs a schema proposal | Each agent gets a stable system voice of the profile language chosen by its id; the user cannot pick it | P2 | The user picks a voice per agent and it is kept |
| S-17 | Stopping a run already using tools when Agent tools is turned off | Rare; the run can be cancelled | A run keeps its tools until it ends | P2 | The switch is checked before every call, with a test |
| S-19 | Flaky frontend tests under full-suite load (RoomsPage/RoomView 1 s timeouts) | Pre-existing (fails 1 of 2 full runs before this phase too) | CI may fail randomly | P1 | Ten consecutive full `pnpm test` runs pass |
| S-20 | Command-layer tests and atomic review snapshot (BR-17 leftovers) | Security closure items, not blocking local use on macOS | See [tools-and-approvals.md](tools-and-approvals.md) | P1 | Items closed in the BR-17 table |
| S-21 | Accessibility pass of the meeting (screen reader order, focus after sending a spoken turn, reduced motion on the speaking bars) | MVP first | Bars honor reduced motion; nothing else was audited | P2 | Manual VoiceOver walkthrough documented, issues fixed |
| S-22 | Measured latency of the voice loop (press → transcript → reply → audio) | Needs real use | Load ~8 s the first time per session (Metal shaders), then ~0.1–0.7 s per sentence on an M1 Max; not measured on other hardware | P2 | Latency table on reference hardware in voice-engine.md |
| S-24 | Tell the user in the room when a request reaches an agent whose tools are blocked by the environment (several eligible tasks, environment not ready) | The switch, cloud and unsupported models are shown since 2026-11-20 (S-18 closed); the environment cases are only told to the model | The agent explains it in its reply, which depends on the model | P2 | The run shows the reason next to the request, with a test |
| S-25 | A short spoken acknowledgement («on it») when an agent starts a long run with tools | Meeting feel, not needed to complete the flow | Silence until the run ends | P3 | The agent says a brief «on it» when a run with tools starts and replies aloud are on |
| S-23 | Tool activity history beyond the last 400 calls of a room | Enough for MVP rooms | Older activity is not shown in the room (it stays audited) | P3 | Paging or per-run loading |

## Future evolution

| # | What | Why later | Priority | Done when |
|---|---|---|---|---|
| F-01 | Cloud STT and TTS (OpenAI, Groq) with explicit consent (D6) | Local works offline and needs no consent | P2 | A provider can be chosen in Settings with consent, and nothing switches silently |
| F-02 | Hybrid AI coordinator and autonomous turns between agents (D2, D3) | Explicit turns are enough for the MVP | P2 | Track R R2 remainder accepted |
| F-03 | Continuous listening with VAD, barge-in and echo cancellation (R5) | Push-to-talk avoids echo and accidental captures | P3 | R5 acceptance |
| F-04 | Simultaneous conversation and interruptions | Out of the MVP by design | P3 | Its own proposal |
| F-05 | Cloud tools consent | Secrets inside files would leave the machine | P2 | Consent proposal approved and built |
| F-06 | `run_command`, `git_commit` and approvals (5B) | Waits for the `approvals` schema | P1 | [approvals-proposal.md](approvals-proposal.md) approved and built |
| F-07 | Advanced audit view of tool calls | The room shows the main operations | P3 | A per-task audit screen |
