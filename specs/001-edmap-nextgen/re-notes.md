# EdMap RE Notes (Track 1)

Living document. Each section captures a question from `ux-spec.md §12`, what we
learned from radare2 / DOSBox-X, and what's still open. RE is lazy — we only dig
when the rebuild can't infer a behavior from public DOOM specs or strings.

## Tooling

- `radare2 6.1.4` — installed via brew. Open with: `r2 -e bin.cache=true -e asm.bits=16 EDMAPSYS.EXE`. The `bin.cache=true` is needed so MZ relocations are applied; without it cross-references through far pointers don't resolve.
- `dosbox-x 2026.03.29` — installed via brew. Use for runtime observation when static analysis stalls. Launch with `dosbox-x -conf <test.conf>` and a stub WAD that exercises the behavior under test (e.g. for "long LineDef": craft a one-room WAD with linedefs of incrementing lengths, run EdMap, watch which warn).

## Binary anatomy

- `EDMAP.EXE` (4 640 B) — Borland TP launcher. Strings: `ERROR:`, `File not found:`, `Path not found:`, `EDMAPSYS.EXE`, `Portions Copyright (c) 1983,90 Borland`. Just shells `EDMAPSYS.EXE`.
- `EDMAPSYS.EXE` (361 984 B) — main app, MZ 16-bit, real-mode x86, single `.text` segment containing both code and Pascal-style string data.
- DGROUP (data group) base ≈ vaddr `0xCD8E` (computed by aligning observed `mov di, imm16` string-pointer loads against known string vaddrs).
- Pascal strings are length-prefixed: a single byte `len` immediately precedes the chars. `WriteString`-style routines take a far pointer to the length byte, e.g. `mov di, off; push cs; push di; lcall <write>`.

## Key globals (by vaddr)

| Addr     | Apparent role |
|----------|---------------|
| `0x2ce2` | Error/warning row counter (incremented when a check emits a row) |
| `0x2ce4` | Error/warning category counter (incremented when a category fires) |
| `0x300a` | Display panel constant (likely a coordinate) |
| `0x900e..0x9010` | UI rendering scratch (panel coordinates) |

## Function map (so far)

| vaddr | role |
|-------|------|
| `fcn.0001584c` | LineDef-check setup / iteration scaffold; initialises counters; allocates panel |
| `fcn.00016248` | Per-LineDef inspector: dispatches the various length / size / texture checks. Calls `fcn.0001584c` and `fcn.00015d9e`. |
| `fcn.00015d9e` | Error display orchestrator. Owns the rendered "Error List" popup and the four LineDef warning strings. |

## Open question 1 — "long LineDef" warning threshold

**Status: not yet pinned to a concrete value.**

What we know from `fcn.00015d9e`:
- The four LineDef-related Pascal strings live consecutively at vaddr `0x15ed5`–`0x15f3e` and are loaded via `mov di, 0x9146 / 0x914f / 0x9151 / 0x9152` (DGROUP-relative).
- `var_18h` (a single-byte local in the orchestrator) selects the message variant: 0 → warning path, non-zero → error path.
- The actual `cmp` against the threshold is *upstream*, not in the orchestrator. The orchestrator just renders the string and increments counters.

What's left: the threshold compare is somewhere in `fcn.00016248` or one of its callees. r2 didn't surface a clean `cmp ax, imm16` against a typical length value (1024, 1500, 2048) in the snippets we walked. Likely the length is computed via `imul + sqrt` (or `imul + isqrt`) before comparison — or compared as `dx:ax` 32-bit since linedef lengths can exceed 16-bit signed range when squared.

**Recommended next step:** DOSBox-X interactive RE.
1. Build a tiny PWAD with a single sector and one LineDef of length 100 units.
2. Open in EdMap, run "Check > LineDefs" — should not warn.
3. Increase length, repeat. Bisect until the warning fires; that's the threshold.
4. Confirm by static lookup of the discovered constant in `EDMAPSYS.EXE`.

This is a 5-minute experiment in DOSBox-X vs. an unbounded r2 archaeology session.

## Open question 2 — texture-align math (F8, Alt-F8)

**Status: not investigated.** Identified strings `Configure Texture Align`, `Adjust X offset?`, `Adjust Y offset?` at vaddr ~`0x1A...`. Function not yet located.

Hypothesis: F8 walks a chain of LineDefs (auto-detected by sharing a vertex) and accumulates X-offsets so the texture appears to span the chain seamlessly. Standard DOOM-mapping math:
```
new_x_offset = previous_x_offset + previous_linedef_length (mod texture_width)
```
We can implement this directly from the spec without RE. Open only if our impl visibly differs from EdMap's behavior.

## Open question 3 — sector resize / rotate (R, Z)

**Status: not investigated.** Strings `+Resize\Sector too large for view; zoom out.` and `C+Rotate\Sector too large for view; zoom out.` and angle units `A:` suggest:
- `Z` (Resize) → uniform scale by user-entered X/Y factor about sector centroid.
- `R` (Rotate) → rotation about sector centroid by user-entered angle.

Implementable from inference. Open only if pivot point or rounding differs.

## Open question 4 — Polygon (Ctrl-P)

Strings: `number of sides`, `radius (vertices)`, `Place center of polygon`. Standard regular-polygon math. Implementable directly.

## Open question 5 — Stairs auto-construct

Strings: step rise / depth / width / count, direction (N/E/S/W). Standard staircase generator. Implementable directly.

## Open question 6 — ADD file format

EdMap-specific. Strings around `Write ADD file`: `These PWADs will be added to the PWAD list at startup, and run with DOOM during Play-map.` Looks like a flat text list of PWAD paths, possibly with a magic header. Need DOSBox-X: write an ADD file with two known PWADs and `xxd` the result.

## Open question 7 — `SESTYLE*` sector style file format

Strings: `Add style`, `Delete style`, `Move style record`, `Styles file corrupt [2]`, `Styles file write error [3]`. Text-based or binary indexed records? Need DOSBox-X: create a few styles, dump file.

## Open question 8 — `MOUSEPIC.DAT`

Cosmetic only — modern OSes own the cursor. **Skip** unless we want pixel-perfect cursor fidelity, which is non-goal.

## How to extend this doc

When picking up an open question:
1. Read the strings cluster around the feature (already extracted in `/tmp/edmap_strings_all.txt`).
2. Find the strings' vaddrs via `r2 -q -c 'izz' EDMAPSYS.EXE | grep <feature>`.
3. Search for `mov di, imm16` patterns where `imm16 = vaddr - DGROUP_BASE`.
4. Walk back from there; the calling function does the work.
5. **In parallel**, run the binary in DOSBox-X and exercise the feature with crafted inputs. The dynamic side often answers the question faster than static disassembly.

Capture findings here under a new section. Don't try to RE everything — prefer
inference from the public DOOM spec when behavior matches. RE is for the
*differences* that make EdMap feel like EdMap.
