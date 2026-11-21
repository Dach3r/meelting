# Product vision

## What it is

A collaborative work platform where the user creates a virtual office with specialized AI agents. Inspired by video conferencing apps (Google Meet), but instead of bringing together only people, it brings together agents that represent the roles of a team. Each agent has its own responsibilities, knowledge, tools and capabilities.

**Main proposition:** turn the traditional interaction with AI assistants into a collaborative, visual and supervisable work experience.

## Problem

Automating work with AI agents today means:

- Manually interacting with different tools and assistants.
- Agent work that is hard to observe and understand.
- No intuitive representation of how several agents collaborate.
- Complex tasks require coordinating tools, knowledge and responsibilities.
- Hard to maintain supervision, traceability and control of automated actions.
- The user must constantly step in to provide context or coordinate.

Meelting offers a centralized environment where agents work as members of a team.

## Value proposition

Meelting is not just a chatbot, a video call tool or a task manager. It is a space where agents can:

1. Receive goals and tasks.
2. Plan how to carry them out.
3. Communicate with each other.
4. Use external tools.
5. Perform authorized actions.
6. Share their activity visually.
7. Request human intervention.
8. Deliver verifiable results.

**Differentiator:** the user observes and directs the work as if coordinating a real team.

## Target experience: the meeting

Meelting feels like a video call with a work team, not like a chatbot. The supervisor opens a room and sees the agents as participants with their state (available, working, speaking, listening). They talk by voice or chat, address an agent by name or the whole team, and one agent answers at a time, aloud and in the chat. When asked, an agent works on a task in its own worktree; the room shows what it is doing and the supervisor reviews the changes without leaving the meeting. The end goal is a team the supervisor can delegate real work to, with control over every change. The current state is in [roadmap.md](roadmap.md#meeting-mvp-product-phase) and the work still needed for a robust product in [stabilization-backlog.md](stabilization-backlog.md).

## Principles

| Principle | Meaning |
|---|---|
| Simplicity | Intuitive, without requiring knowledge of multi-agent architectures. |
| Transparency | The user understands what the agents are doing, what they completed and what they need. |
| Human control | Sensitive, destructive or externally impactful actions respect permissions and require approval when appropriate. |
| Collaboration | Agents share context, delegate and coordinate tasks. |
| Progressive autonomy | Start with controlled capabilities; autonomy grows as reliable execution and supervision mechanisms exist. |
| Real results | The value is completing verifiable work. The interface does not replace real execution. |
| Premium experience | Minimalist, modern, clean interface focused on productivity. |
| Hybrid operation | No backend of its own or account; data lives on the machine. The user combines cloud and local models and engines, with no mandatory preference. Local things work offline; whatever depends on the cloud reports when it is unavailable, and the provider is never switched without consent. |
| Conversation like in a meeting | The user talks or writes with their agents as in a work meeting, directing who speaks. See [voice-meetings.md](voice-meetings.md). |

## Fundamental principle

> Meelting does not aim to simulate that an artificial intelligence is working. It aims to let artificial intelligences really work, while the user observes, collaborates and stays in control.

The virtual office, the avatars and the rooms are the interface that makes that work understandable. The real product is the ability to delegate, coordinate, supervise and complete real tasks through AI agents.
