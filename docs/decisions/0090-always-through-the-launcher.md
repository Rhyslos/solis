# 0090. Always through the launcher

- **Status:** Accepted (the project lead, 2026-11-06: "We go through the launcher, always, no 2 ways around it"; changes 0086 and 0045). Changed by [decision 0099](0099-the-g-layout.md): the launcher becomes a File page in Shared 1b, and whether it keeps its small window is asked then; always starting on the list of projects, and never reopening one, stand.
- **Date:** 2026-11-06

## Problem

Solis asked about a newer build on its splash screen (decision 0045), then reopened the project open last, so the launcher (decision 0086) showed only when no project opened. The project lead wants to choose the project every time, in a small window, the way Unity Hub and Unreal's project browser work.

## Options

- Where the update question goes: **in the launcher, as a bar** (chosen), or kept on the splash.
- "Carry on without a project": removed, or **kept** (chosen: handy for looking at a model without a project).
- Closing the editor's window: **back to the launcher** (chosen), or quit.

## Decision

- **Solis always starts on the launcher**, in a small window (1100 × 700, centred). The last project isn't reopened. Opening a project grows the window to the editor's size (as it was last left, remembered per computer), centred.
- **Closing the editor's window goes back to the launcher**, the project staying open behind it ("← Back to" returns). Closing the launcher, File > Quit and Restart to update quit, asking about unsaved work first.
- **No update question as Solis starts:** GitHub is still asked (unless turned off), and a newer build shows as a bar over the projects: **Update now**, **See what's new**, **Not now**; then *Downloading…* and **Restart to update**. A build skipped with Skip this version gets no bar.
- The launcher's "Show this page when Solis starts" setting goes: it always shows.

## Consequences

- One way into a project, so what's open is always a choice. A project used every day is one double-click, at the top of the list.
- The splash still shows for a moment as Solis starts; it no longer waits for GitHub.
