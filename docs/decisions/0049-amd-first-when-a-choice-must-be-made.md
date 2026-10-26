# 0049. AMD first, when a choice must be made

- **Status:** Accepted
- **Date:** 2026-10-26

## Problem

Graphics cards from different makers are fast at different things. Most of the time one way of doing something is fast on all of them. Sometimes it isn't, and a choice has to be made: a workgroup size, a technique, a format, which of two graphics backends Solis uses. Solis had no rule for those choices.

## Decision

Chosen by the project lead: **Solis supports every card it runs on, and everything it makes stays solid, robust and fast on NVIDIA and Intel too. But when a choice has to be made between tuning best for one maker or another, Solis tunes for AMD.** The aim is for Solis to be the engine that runs best on AMD cards.

In practice:

- **AMD's cards are the reference.**
  - The scale check's numbers that decide a gate come from an AMD card.
  - Phase gates are tried on one where possible.
  - A regression on another maker's card still counts, as it does today.
- **Shaders are written for how AMD cards run work.**
  - Compute groups are sized in multiples of 64. That suits AMD's 64-wide and 32-wide waves, and NVIDIA's 32-wide ones too.
  - Memory is laid out for AMD's caches.
  - Wave operations are used only where wgpu has them on every card, with a plain fallback.
- **Techniques that suit AMD are preferred,** for example:
  - software and compute methods over ray-tracing hardware (decision 0046's light plan already leans this way);
  - AMD's open-source FidelityFX libraries (MIT) where one fits, such as:
    - Brixelizer GI for real-time bounced light;
    - FSR for upscaling;
    - CACAO for ambient occlusion;
    - SPD for the downsampling that bloom and auto exposure need.

  Each is still measured against the alternatives.
- **The graphics backend is chosen by measuring.** wgpu can draw through DirectX 12 or Vulkan on Windows. Solis keeps a setting, and the default on AMD cards is whichever measures faster there.
- **Profiling uses AMD's tools** (Radeon GPU Profiler) alongside Solis's own frame profiler.
- **Nothing maker-only becomes required.**
  - A feature that only runs well on one maker's cards is optional, behind a quality level.
  - A feature that runs on one maker's cards only is not taken at all.

## Consequences

- **An AMD card is needed for gates and the scale check.** If the project lead's PC has one, it is the reference machine. If not, the gates note which card they ran on, and an AMD card is borrowed or bought when a phase's tuning depends on it.
- **CI can't test AMD:** GitHub's machines have no AMD graphics. Correctness is still tested everywhere; speed on AMD is measured by hand at each phase's end, as the scale check already is (decision 0011).
- **The Phase 8 and 9 plans** follow this rule: Brixelizer GI and the FidelityFX pieces are first in line, and wave sizes follow the list above.
