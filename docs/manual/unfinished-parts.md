# Unfinished parts

> Parts of the editor that aren't built yet are shown in grey with a small square mark, and one setting hides them all.

## When to use it
Solis's interface is being rebuilt to one design, and that design is larger than what Solis can do today: it has places for things that come in later updates, such as materials, effects and a built-in code view. Rather than leave holes, the editor shows each of those places where it will be, marked so you can tell it apart from what works. This is called an [unfinished part](concepts.md#the-window).

Read this page when you see a grey control with a small hollow square beside it and wonder why a click does nothing, or when you'd rather not see any of them.

**Why show them at all?** So the editor's layout stays the same as it grows: what you learn now is where things will still be, and you can see what's coming and roughly when. If you'd rather work with only what works today, hide them: nothing you can use disappears with them.

Today there is one: **Code editor split**, on the Appearance page of **Edit > Settings…**. More appear as the new interface's parts arrive (its [ribbon](concepts.md#the-window) and its [shelf](concepts.md#the-window)), each in its own place.

## How to use it
1. **Recognise one.** An unfinished part has three signs, always together:
   - its words are a quieter grey than a working control's, and its icon fainter still;
   - a small **hollow square** sits at its top right corner (on a button) or just after its words (on a tab, a row or a setting);
   - it doesn't light up when the pointer is over it.
2. **Find out what it will be.** Rest the pointer on it. A note says its name, what it will do in one sentence, and which step of the [roadmap](../../ROADMAP.md) builds it, for example: *Not built yet: Code editor split. Shows game code next to the viewport when a code file is open. Not planned yet.* "Not planned yet" means the design has a place for it but no step of the roadmap builds it so far.
3. **Clicking it does nothing**, on purpose. The right-hand end of the status bar says so for three seconds (*Code editor split isn't built yet (not planned yet)*), and that's all: no window opens and nothing changes.
4. **Hide them all:** open **Edit > Settings…**, and on **Appearance** switch on **Hide unfinished parts**. Or press **Space** and type `hide unfinished`. Every unfinished part goes, and what's left closes up, so there are no gaps where they were.
5. **Show them again:** switch it off in the same place, or press **Space** and type `show unfinished`.

## Settings
| Setting | What it does |
|---|---|
| Appearance > Hide unfinished parts | Off (the standard): unfinished parts are shown, marked. On: none is shown anywhere, and the things around them close up. It's remembered next time you start. **Reset this page** turns it off again. See [Settings](settings.md). |

## Shortcuts
| Action | Shortcut |
|---|---|
| Hide or show unfinished parts | `Space`, then type `unfinished` and press `Enter` |

## Tips and common problems
- **Problem:** I clicked a button and nothing happened. **Why:** if it's grey with a small hollow square, it's an unfinished part: it marks where a feature will be. **Fix:** rest the pointer on it to read what it will do and when. For what can be done today, the [manual's list of pages](SUMMARY.md) covers everything that works.
- **Problem:** is a grey control broken, or switched off? **Why:** neither is the same as unfinished. A control that works but can't be used right now (**Reset this page** with no project open, say) is faded and has **no** square mark; resting the pointer on it says what to do first. An unfinished part always has the mark. **Fix:** look for the square.
- **Problem:** something I saw before is gone. **Why:** **Hide unfinished parts** may be on: it hides only what never worked. **Fix:** switch it off on **Edit > Settings… > Appearance** to see the marked places again.
- **Problem:** I can't find an unfinished part with the search. **Why:** the search (**Space**) lists only what can be done. **Fix:** nothing to do: when a part is built, it joins the search.

## Related
- [Settings](settings.md): the Appearance page and its other settings.
- [Interface tour](interface-tour.md): the editor's panels and bars as they are today.
- [Concepts](concepts.md): [unfinished part](concepts.md#the-window), [ribbon](concepts.md#the-window), [shelf](concepts.md#the-window), [accent colour](concepts.md#the-window).
