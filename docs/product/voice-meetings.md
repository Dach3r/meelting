# Conversational meetings with agents

> **Status:** proposal, **not implemented** except for the text-directed conversation (R1, ✅ implemented) and what is marked "✅ implemented". Depends on `AgentRun` (Phase 3, not built). Decisions already made by the product owner are in [decisions.md](decisions.md); everything else is marked **pending**. No schema change is approved by this document.
>
> Technical documents: [conversation engine](../architecture/conversation-engine.md) and [voice engine](../architecture/voice-engine.md). Current rooms: [rooms.md](rooms.md). Execution: [agent-execution.md](agent-execution.md).

**Problem:** today a room is a persistent chat where only the user writes. Working with several specialized agents requires being able to address one of them, the team, or ask them to discuss, and to do it by speaking too, without losing control or paying for responses nobody asked for.
**Goal:** entering a room should feel like joining a work meeting (Meet or Teams style) where the other participants are AI agents: the user speaks or writes, the agents answer with text and voice, and everything shown corresponds to real states.
**User story:** As the owner of a workspace, I want to gather my agents in a room, talk to them and hear their proposals, so I can discuss decisions with my AI team as I would with people, while keeping control over who speaks and what gets executed.

## 1. Principles

- **The user leads the meeting.** They always have priority: they can interrupt, stop and change topic at any time.
- **Nothing simulated.** "Speaking", "thinking", "unavailable" or "error" reflect real core states. No activity animations without activity.
- **Text is the source of truth.** Voice is a form of input (transcribed to text) and output (synthesized from text). The whole meeting can be used without voice.
- **Conversation ≠ authorization.** A spoken or written sentence never authorizes running commands, writing files or modifying repositories; that stays under the approval rules of Phases 4–5.
- **Bounded cost and turns.** No message wakes up every agent by default, and no agent can start turns on its own.
- **Hybrid operation.** Each agent uses its own provider and model (cloud or local); voice engines are chosen separately. They can be mixed in the same meeting. See §7.

## 2. What changes compared to the current room

| Concept | Today (✅ implemented) | With this proposal |
|---|---|---|
| Room (`rooms`) | Persistent: name, participants, history | Same. Still the unit of context |
| Messages (`messages`) | Immutable, from the user; `message:created` | Same. Agent messages are added via `AgentRun` |
| Meeting session | Does not exist | **Temporary** core state while the room is open; leaving deletes nothing |
| Recipient | Does not exist (the "Ask" selector arrives with `AgentRun`) | Per intervention: one agent, several, the team or a discussion |
| Voice | Does not exist | Push-to-talk input and per-agent voice output, optional |

## 3. Conversational modes

**Approved decision:** the mode is stored neither in the room nor in the session; it is resolved **per intervention**, based on the recipient. Changing mode means choosing another recipient.

| Recipient | Mode | Who responds |
|---|---|---|
| None | Message without a response | Nobody (current behavior). See "default recipient" below |
| One agent (selector or mention) | **Directed** | Only that agent |
| Several named agents | **Round** | Each one once, in the order named |
| "Team" | **Meeting** | The coordinator chooses who contributes (§4) |
| "Discussion" between named agents | **Discussion** | The named agents alternate up to the limit (§4) |

- **Directed mode** is the recommended starting mode and the first one built: the user controls who responds and there are no unnecessary runs.
- **Asking the team** is only possible by choosing "Team" (selector or the vocative «Equipo», «Team», «todos»). It is never activated by default.
- **Default recipient (✅ implemented, D19):** if the user names nobody and the last agent that responded successfully in the room during this app session is still participating, the proposed recipient is that agent ("Replying to Backend"), visible and changeable before sending. It handles follow-ups such as "Explain the PKCE part better". Without a previous session, there is no recipient.

### Mentions and ambiguity

- **Text:** `@` opens the participant selector; the agent's name at the start of the sentence is also recognized («Backend, …»).
- **Voice:** the transcript is parsed the same way: a leading vocative («Backend, ¿cómo…?», "Backend, how…?"), participant names or roles, and «equipo» ("team").
- Matching ignores case and accents and accepts the agent's name or role (for example "Backend" matches "Backend Developer" if it is unique).
- **Ambiguity:** if a reference matches two or more participants (two "Backend" agents), the core does not choose: the intervention stays as a draft and the UI asks to choose among the candidates. A "probable" agent is never run.
- A reference to someone who is not participating in the room is not resolved (agents from other rooms or workspaces do not exist for the meeting).

## 4. Turn coordination

**Approved decisions:** a **hybrid** coordinator with **rules first and optional AI**; **bounded discussions** with a maximum of **4 automatic interventions**, only if the user asks for them.

- The rules (deterministic, in Rust) resolve: explicit recipient, mentions, rounds, limits, availability, cancellations and end of discussion by limit.
- Only for "Team" is a **suggestion** requested from a coordinator model chosen by the user in Settings. The suggestion is an ordered list of participants that the rules validate (only available participants, no repeats, within the limit). If no coordinator model is configured, or it is unavailable, fails or returns something invalid, the **fixed round** (room order) is used, with the option for each agent to pass.
- **Passing (pending, recommended):** in team mode each agent can reply that it has nothing to add; that reply is not published as a message. It prevents everyone from always speaking.
- **Limit:** one user intervention produces at most 4 automatic agent responses. Applying it to "Team" too (besides discussion) is **pending, recommended**.
- **End of a discussion:** the limit is reached, an agent passes, the user intervenes or presses "Stop", or (if there is a coordinator model) it indicates that the discussion is over. When it ends, the turn returns to the user.
- **Agents do not summon others.** An agent writing "@QA, what do you think?" does not run QA. In a later stage, the UI could offer it as a suggestion the user accepts (**pending**).
- The coordinator does not run tools, does not change limits or providers, and cannot create approvals.

## 5. Scenarios

The sequence diagrams for each scenario are in [conversation-engine.md](../architecture/conversation-engine.md#8-sequence-diagrams).

### A. Directed conversation with one agent

A room with Product Owner, Backend Developer and QA Engineer. The user holds down the microphone button and says "Backend, how would you implement OAuth authentication in this project?".

1. While holding, the core captures audio and the UI shows "Microphone on".
2. On release, the intervention ends (push-to-talk: no end detection needed).
3. The configured STT engine transcribes; the UI shows "Transcribing…".
4. The core resolves the recipient: "Backend" → Backend Developer (unique).
5. The user's message (transcribed text) is saved and an `AgentRun` is created for Backend.
6. The context includes Backend's instructions, participants, recent room history and the question.
7. Backend's card switches to "Thinking…"; the transcript shows the provisional text token by token.
8. When generation finishes, the response is saved as a Backend message.
9. The TTS engine synthesizes the response with Backend's voice and plays it; its card shows "Speaking".
10. If the user presses the microphone again, the audio stops immediately (§6).
11. When playback ends, Backend returns to "Available".

Afterwards, "Explain the PKCE part better" (no name) goes to Backend through the default recipient (pending) and its context already contains the previous answer: the conversation does not restart.

**Exceptions:** microphone without permission, empty audio or silence (nothing is sent), STT unavailable (writing is offered), Backend unavailable (§7), provider error (status line with the reason, the request can be repeated).

### B. Question to the whole team

"Team, what do you think about implementing authentication with OAuth?"

1. Vocative "Team" → meeting mode.
2. The rules filter available participants. If there is a coordinator model, it suggests who contributes and in what order (e.g. PO → Backend → QA); otherwise, a fixed round in room order.
3. Responses are generated **in sequence**: each agent sees what the previous ones said (Backend can respond to the PO's proposal). Generation of the next one can start while the previous one's voice is playing; voices never overlap.
4. An agent with nothing to add passes (pending) and nothing is published.
5. "I like it. Backend, develop your proposal" → directed mode to Backend, with everything above in its context.

### C. Discussion between agents

"Product Owner and Backend, discuss how we should implement the authentication system."

1. A discussion between PO and Backend is recognized: through the "Discussion" recipient in the selector or through a verb from the localized list («discutan», «debatan», «discuss», «debate»). Exact detection **pending**.
2. They alternate PO → Backend → PO → Backend: at most 4 interventions.
3. Each intervention is an `AgentRun` whose origin is the user's intervention: no agent launches turns.
4. The user can intervene at any time: their intervention discards the discussion's pending turns and is handled first.
5. When the limit is reached, the UI shows "Discussion ended (4 of 4)" and offers "Continue the discussion" (a new user intervention, with another budget of 4).

### D. Human interruption

While Backend is speaking, the user presses the microphone: "Wait, first explain how the flow would work".

1. On press, Backend's audio stops immediately and pending audio is discarded.
2. Planned turns that have not started are paused. They are discarded when the new intervention is sent.
3. If a generation was in progress, it is cancelled when the new intervention is sent (**pending, recommended**; alternative: let it finish silently).
4. The new intervention is processed with its recipient.

**What stays in the history (approved decision):** if generation finished, the full message stays saved even if the audio was cut off. During the session, the UI marks it "interrupted". That mark lives only in memory and is not persisted. If generation was cancelled, there is no message: the run ends `failed` with `error_kind = cancelled`, as `AgentRun` already defines.

| Situation | Message persisted? | Visible mark |
|---|---|---|
| Generated and fully heard | Yes | — |
| Generated, audio interrupted | Yes, full text | "Interrupted" (session only) |
| Generated, audio never played (discarded) | Yes, full text | "Not played" (session only) |
| Generation cancelled | No | "Cancelled" status line |
| Generation failed | No | Status line with the reason |

When the room is reopened, it is not known which part was heard: this is an accepted limitation.

### E. Unavailable participants

One agent uses Ollama, but Ollama is not running. Another uses a cloud provider without a key.

- Each card shows its real state with the reason: "Ollama is not running" and "Anthropic key missing".
- The meeting continues with the available agents. An unavailable agent is not scheduled in rounds or discussions; if a question is addressed to it, the UI warns before sending; if the user sends anyway, the intervention is saved and the run fails immediately with its reason (`provider_not_configured`, `provider_unavailable`…), as [agent-execution.md](agent-execution.md) already defines.
- Alternatives offered: "Retry" (checks again), "Configure key", "Open Ollama" (as a hint only) or addressing the question to another available agent in the room.
- The provider or model is **never** switched automatically. An explicit, user-configured fallback policy remains a **pending** evolution.
- Today an agent cannot change provider or model after being created: `update_agent` only edits name, role, description, instructions and team. Offering "Change the agent's model" depends on adding that edit.

### F. Offline meeting

Without internet, with Ollama or LM Studio running:

- Workspaces, teams, rooms and history remain available.
- Agents with a running local model respond.
- Voice works if the chosen STT and TTS are local and their models are downloaded.
- The coordinator's deterministic rules work. The AI suggestion only works if the coordinator model is local; otherwise, the fixed round is used.
- Agents and engines that depend on the cloud are shown as "Requires connection". Their response is never simulated.
- No external call is made without an explicit user configuration.
- When the connection returns, "Retry" restores their availability.

### G. Meeting with real work (evolution, phases 4–5 and later)

"Backend, review the authentication module and propose a solution."

- **Until Phase 5:** the agent answers verbally with what it knows from the context. It does not read files.
- **Afterwards:** a **task** (agent, repository, base branch) is created from the meeting with its worktree. The agent requests permissions according to [agent-execution.md](agent-execution.md#tools-and-approvals), and the shared screen shows the real terminal, diff and artifacts. The agent can explain its progress by voice.
- Creating a task or approving a command are **explicit UI actions**, never the result of a spoken sentence or of another agent's message.

## 6. Voice

**Approved decisions:**
- **Push-to-talk** in the first version.
- **Configurable engines**, local or cloud, with no mandatory preference.
- **Per-agent voice** independent of its LLM provider, stored in a separate table (`agent_voices`). The exact schema is pending approval.

- **Input:** hold the button (or the shortcut within the window) to speak. Pressing stops the agents' voices. Since the microphone is not open while they speak, echo is avoided without echo cancellation in v1.
- **Transcription:** with the recipient resolved, it is sent automatically. With an ambiguous or missing recipient, it stays as an editable draft in the composer. Both rules are **pending, recommended**.
- **Output:** each response is synthesized with the agent's voice. **Only one voice plays at a time**; the others wait in a queue in the logical order of the conversation.
- **Controls:** mute the agents (the conversation continues in text), captions, stop, volume and, if feasible, input and output device.
- **Replay:** any agent message can be heard again (it is synthesized from the saved text).
- **Languages:** Spanish and English. By default STT uses the profile language. Each agent needs one voice per language: **pending** whether `agent_voices` is keyed by agent and language.
- **Audio:** no recordings are saved. Microphone audio lives in memory until transcribed and is then discarded. Synthesized audio is not saved. Policy **proposed, pending confirmation**.
- **Transparency:** with a cloud engine, the UI indicates that audio leaves the machine for that provider. Synthetic voices are identified as AI-generated, as some providers require.

Technical detail and engine comparison: [voice-engine.md](../architecture/voice-engine.md).

## 7. Availability and hybrid operation

Meelting is a desktop app with no backend of its own. It works with cloud services and with models and engines installed on the computer, and the user chooses what each agent and each voice engine uses. **There is no mandatory preference for local or cloud.**

Availability states of an agent or a voice engine:

| State | Meaning | Example |
|---|---|---|
| Requires configuration | The minimum needed to use it is missing | Cloud provider without a key; local voice model not downloaded |
| Configured | Has what it needs, not checked yet | Key saved, not verified in this session |
| Available | Checked and responding | Ollama running with the agent's model |
| Unavailable | Checked and failing | Ollama off; model missing; key rejected |
| Requires connection | Depends on internet and there is no connection | Agent using Claude without network |

- A saved key does not mean "Available".
- States are computed from real checks and from run results. "Retry" checks again.
- **Fallback:** the provider or model is never switched silently. If the connection is lost, the Claude agent becomes "Requires connection": the user is informed, retrying is allowed and available agents in the room are offered. An explicit fallback policy is **pending**.

## 8. Visual experience

Consistent with Grafito: the same tokens, components and tone as the current room. No decorative controls: each control corresponds to a real function of its stage.

```
┌──────────────────────────────────────────────────────────────────────┐
│ ← Rooms   OAuth authentication   ● In meeting   4 participants   ⋯   │  header
├───────────────────────────────────────────────┬──────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐        │ Conversation         │
│  │  (You)  │  │   PO    │  │ Backend │        │ ─ messages ─         │
│  │ Superv. │  │Availabl.│  │▶Speaking│        │ Backend (typing)     │
│  └─────────┘  └─────────┘  └─────────┘        │ …provisional text…   │
│  ┌─────────┐                                  │                      │
│  │   QA    │      [caption of the speaker]    │ To: [Backend ▾]      │
│  │Thinking…│                                  │ [composer       ] ⏎  │
│  └─────────┘                                  │                      │
├───────────────────────────────────────────────┴──────────────────────┤
│ 🎙 Hold to talk   🔊 Voices   CC Captions   ■ Stop   Leave   ⚙      │  bottom bar
└──────────────────────────────────────────────────────────────────────┘
```

- **Header:** back, room name, meeting state ("In meeting" / "Stopped"), number of participants and menu (audio settings, delete room). Delete room is in the bottom bar today; moving it to the menu is proposed.
- **Cards:** avatar, name, role and state. "Speaking" has a ring, icon and text (not only color). "Thinking…" only appears with an active run. "Unavailable" shows the reason. With more than 6 agents, the speaker moves to the foreground.
- **Side panel:** persisted conversation and provisional text (styled as provisional), "interrupted" or "not played" marks, status lines for each run, composer with recipient ("To: Backend · Team · Discussion").
- **Bottom bar:** push-to-talk microphone, mute voices, captions, "Stop" (cancels everything pending in the room), leave the meeting and audio settings. Each control appears in the stage that makes it real.
- **Responsive** (minimum window 800×600): at large widths, grid and fixed side panel (as today). At smaller widths, the panel collapses into a drawer or "Participants · Conversation" tabs. Each area has its own scroll and the composer always stays visible.
- **Keyboard:** everything is operable by keyboard. The push-to-talk shortcut only works with the window focused and outside the composer (no global shortcut). Specific shortcut **pending**.
- **Accessibility:** new messages are announced with `aria-live="polite"` and errors with `assertive`; states have text; `prefers-reduced-motion` is respected; captions are always available and every spoken action has a written equivalent.

### Empty and error states

| Case | Experience |
|---|---|
| Room without participants | Writing is possible; voice and recipient controls are disabled and the user is invited to create a room with agents (a created room is not edited) |
| Room without messages | Suggestion of how to address an agent or the team |
| Microphone without permission | Microphone button disabled with "Microphone permission denied" and how to grant it in the OS; text remains available |
| No model available | Cards with the reason; the composer saves messages without a recipient |
| STT not configured | The microphone leads to Settings › Voice |
| TTS not configured | Voices disabled, the conversation continues in text |
| Meeting stopped | "Stopped"; nothing pending; the user can keep writing |
| Agent unavailable | State and reason on the card; excluded from rounds |

## 9. Acceptance criteria (per stage; see [roadmap.md](roadmap.md))

**Directed conversation (text)**
- [ ] An intervention can only be addressed to agents participating in the room; a foreign id or one from another workspace fails without saving anything.
- [ ] An ambiguous mention runs nobody and asks to choose.
- [ ] Each agent's context includes the other agents' messages with their name and role, and keeps its own instructions.
- [ ] An agent's message is never presented or sent to the model as if it came from the user.

**Coordination (text)**
- [ ] An intervention produces at most 4 automatic responses.
- [ ] No agent message creates a run.
- [ ] A user intervention discards the pending turns of the previous one.
- [ ] "Stop" cancels everything pending in the room idempotently; a late event reactivates nothing.
- [ ] Without a coordinator model, or if its suggestion is invalid, the fixed round is used.
- [ ] An unavailable agent is not scheduled.

**Voice output**
- [ ] Two voices never play at the same time.
- [ ] Muting voices does not alter the text conversation.
- [ ] Leaving the meeting, switching workspace or closing the window stops audio within a target to be measured.
- [ ] No synthesized audio is saved.

**Voice input**
- [ ] The microphone is only open while held down, and the UI indicates it.
- [ ] Pressing the microphone stops the agents' voices.
- [ ] Empty audio or silence creates no messages.
- [ ] Without permission, device or STT, text keeps working and the cause is reported.
- [ ] No microphone audio is saved.

**Cross-cutting**
- [ ] Everything in `es` and `en`; accessible states that do not depend on color.
- [ ] No keys in events, logs or transcripts.
- [ ] With only local engines and models available, the meeting works offline.

## 10. Proposed MVP

**First demonstrable spoken meeting** (at the end of stage R4 of the [roadmap](roadmap.md#track-r--conversational-meetings)): directed mode and team mode by text with the hybrid coordinator, voice output with one engine per platform and push-to-talk input with one STT engine. Out of the MVP: continuous listening, streaming sentence-level TTS, echo cancellation, speech-to-speech and tools.

## 11. Out of scope

- Real video, cameras, WebRTC between people, additional human participants.
- Remote sync, accounts, our own cloud services.
- Persisted audio recordings.
- Long-term memory and automatic summaries (their behavior and persistence pending approval).
- Agents that speak spontaneously or summon each other.
- Automatic switching of provider or model.

## Dependencies
- `AgentRun` with streaming and cancellation ([agent-execution.md](agent-execution.md)).
- Rooms and messages ✅ ([rooms.md](rooms.md)).
- `settings` table (planned) for the coordinator model and voice preferences; `agent_voices` table (proposed).
- Audio and voice spikes ([voice-engine.md](../architecture/voice-engine.md#9-spikes)).

**Priority:** Should Have (after `AgentRun`).
**Success metric:** percentage of rooms with at least one intervention to "Team" or to several agents, rate of voice versus text interventions, rate of interruptions and of discussions that reach the limit, and time to first audio (measured locally).
