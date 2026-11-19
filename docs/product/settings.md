# Settings

Preferences of the installation. They are global (one row in `profile`), not per workspace, and every change is saved at once with no save button. Provider keys keep their own sidebar view ([decision D26](decisions.md)).

## Layout

A left navigation that lists the sections, each with a count of its settings, with no title or group headings. The content shows the active section as one card per group of settings, with the group title above the card. A new setting is added to a group, a new group to a section and a new section to the list, so the screen grows without changing its structure.

| Section | Group | Setting | Values (default) |
|---|---|---|---|
| General | Language and region | Language | `es`, `en` |
| | | Time format | Automatic, 12 h, 24 h (automatic) |
| | Startup | Open at login | on, off (off) |
| Appearance | Theme | Theme | System, Light, Dark (System) |
| | Interface | Interface size | 90, 100, 110, 125, 150 % (100) |
| | | Transition effect | On, Off (On) |
| | | Reduce motion | Automatic, On, Off (Automatic) |
| Notifications | Alerts | Run finished | on, off (on) |
| | | Run failed | on, off (on) |
| | Sound | Sound | on, off (on) |
| Environments | Location | Environments folder | native folder dialog ([D39](decisions.md)) |
| Advanced | Data | Data folder | opens the folder in the file manager |
| | | Provider keys | information only: they live in the operating system keychain |

## Behavior

- **Time format:** `Automatic` uses the language's own convention; `12 h` and `24 h` force it everywhere a time is shown.
- **Open at login:** the state is read from and written to the operating system (login item on macOS, startup entry on Windows, autostart file on Linux). It is not stored in `profile`, so it cannot drift from what the system really does.
- **Reduce motion:** `Automatic` follows the system preference; `On` and `Off` override it. It affects the animations that already honor `motion-reduce` (equalizer, fade, flyout, transition overlay).
- **Notifications:** a system notification is shown when an agent run reaches `succeeded` or `failed` and the Meelting window is not focused. Title is the agent's name and the body a short sentence in the profile language. Runs cancelled by the user or expired at startup never notify. Each event and the sound have their own switch. The notification permission of the operating system still applies.
- **Data folder:** opens the application data folder, where the local database lives. Nothing in it is edited by the app from this screen.

## Not built

Keyboard shortcuts, density, quiet hours, a debug log, «Reset settings» and «Clear history» were considered and left out (see [decisions.md](decisions.md), D50).
