# 0132: Properties' cards in one column, and folding

- **Status:** Accepted (the project lead, 2026-11-19). This changes the G layout's Properties (spec 199-213), where cards sat side by side as many as fit.
- **Date:** 2026-11-19

## Problem

When Properties was made wide, or pulled out of its column, its component cards went side by side. The project lead found that disorganised and ugly, and asked for every card to stack one under another across the whole editor, with each section foldable.

## Decision

- Properties' cards are always one column, one card under another, however wide the panel is.
- Each card's title row starts with a chevron. A click on it, or on the card's name, folds the card to its title and back.
- A card stays folded for that kind of component (all Light cards fold together), and stays folded from one session to the next.
- The "What game code hears" card and Hints have no title row, so they don't fold.

## Consequences

- Wide Properties panels leave room at the side instead of filling it with cards.
- The approved pictures showing Properties were taken again.
