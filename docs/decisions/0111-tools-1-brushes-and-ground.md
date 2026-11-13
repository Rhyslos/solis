# 0111: Tools 1: editable brushes, and a sculpted ground with caves

- **Status:** Accepted (the project lead, 2026-11-13; carries out [0108](0108-shape-brushes-and-sculpting-come-next.md); the ground's storage and drawing are settled by Tools 1b's study, rule 22)
- **Date:** 2026-11-13

## Problem

Decision 0108 put shape brushes and world sculpting next. What a brush is, how it's placed and edited, what sculpting shapes, how the ground looks, and their keys had to be chosen before planning (rule 17 for the keys).

## Options

For each, the project lead was given a recommended choice and alternatives:

- **A brush:** shapes placed and resized only (recommended); shapes that can also cut; shapes that become models. The project lead chose a fourth: **shapes edited point by point, edge by edge and face by face, with extruding**.
- **Placing:** draw it out (recommended); click to drop; both.
- **Sculpting:** a height map (recommended, fastest); a ground with caves and overhangs.
- **The ground's look:** smooth (recommended); blocky; smooth with sharp edges. The project lead chose all three, as a setting per ground.
- **Painting materials:** later (recommended); in this phase.

## Decision

- **Brushes are polygon meshes edited in the editor:**
  - Each is an entity with a **Brush** component holding its mesh, saved in the area file, solid to physics.
  - The mesh core is Aventail's (decision 0007), copied in as a `mesh` crate with Solis's limits.
  - **B** opens the list over the 3D or 2D view, each **B** moves down, **Space** confirms while it's open, **Esc** closes (0108).
  - A brush is **drawn out**: footprint, then height, then click.
  - **Tab** toggles editing its insides; **1** points, **2** edges, **3** faces.
  - **E** extrudes (move, click; Esc or right-click gives up), and so does **Shift + drag** on the move gizmo.
- **The ground has caves and overhangs:**
  - One per area, in chunks, made on workers.
  - Its **look is a setting per ground:** Smooth, Blocky or Sharp.
  - It is **sculpted** with a round brush: left-drag adds, Ctrl + left-drag digs, Shift + left-drag smooths, [ and ] its size, Shift + [ and ] its strength; Flatten as a tool.
  - **Materials are painted** onto it with the same brush, in this phase.
- **Two phases:** Tools 1a (brushes, five milestones) and Tools 1b (the ground, a study and five milestones), each with a gate ([the plan](../plans/tools-1.md)).
- **Mocked up first:** the tools' screens are approved before they are built.

## Consequences

- Tools 1 is bigger than 0108 foresaw; the rest of 2D moves back by two phases, not one.
- Physics gains triangle-mesh colliders, and the renderer meshes made in the editor.
- The ground's storage, its file and how each look is drawn are chosen by measuring competing approaches (rule 22) in Tools 1b's study, each in its own decision.
- E, Tab and 1 to 3 are taken while editing a brush; Shift + gizmo drag means extrude only on a brush's picked faces or edges.
