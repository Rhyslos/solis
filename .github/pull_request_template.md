## What this changes

<!-- One or two sentences, in plain words. Which roadmap milestone does it complete or advance? -->

Milestone:

## Code tests (author sign-off)

<!-- What tests were added, what they cover, and the result of the full suite and the other checks. -->

- Tests added:
- Full suite result:
- Other checks (fmt, clippy, deny, GPU tests, window test):
- Not testable automatically:

## Documentation updated

<!-- Tick what changed, or explain why nothing needed to. -->

- [ ] Code comments
- [ ] Crate README(s)
- [ ] ARCHITECTURE.md
- [ ] CONTRIBUTING.md
- [ ] DESIGN.md or ROADMAP.md
- [ ] Decision record
- [ ] User manual (docs/manual)
- [ ] CHANGELOG.md
- [ ] README progress tracker (milestone completed)
- [ ] RENAMING.md (the program's name appears somewhere new)
- [ ] No documentation change needed, because:

## How to test (project lead sign-off)

<!-- Step-by-step checks for what automated tests cannot cover: feel, looks.
     Written for a non-programmer. -->

1.

## Checklist

- [ ] Does this change a file format, a save, or what game code can call? Then it brings its upgrader and an example of the new version (`tests/fixtures/`), updates `docs/game-code-interface.txt`, and says so under "Breaking" in CHANGELOG.md (CONTRIBUTING.md, rules 23 and 24).
- [ ] All CI checks are green
- [ ] Follows the ground rules in CONTRIBUTING.md
