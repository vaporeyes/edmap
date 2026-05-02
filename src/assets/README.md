# Fonts

The app loads the first font found from this priority list and uses it for
both `Proportional` and `Monospace` families:

1. `PxPlus_IBM_VGA_9x16.ttf`  — primary; VileR's pixel-perfect IBM VGA 9×16 BIOS bitmap (CC-BY-SA-4.0)
2. `PxPlus_IBM_VGA_8x16.ttf`  — secondary
3. `Px437_IBM_VGA_9x16.ttf`   — basic CP437-only variant
4. `roboto.ttf` / `Roboto-Regular.ttf`  — modern fallback

The IBM VGA 9×16 face is the canonical 1990s DOS look — same glyphs the user
would have seen at the C:\\ prompt or in any Borland-era text-mode editor.
The font is monospaced by design, so coordinate columns and texture-name
tables line up correctly without needing a separate monospace font.

## Where to get them

- **PxPlus / Px437**: https://int10h.org/oldschool-pc-fonts/ (Ultimate Oldschool PC Font Pack v2.2)
- **Roboto**: https://fonts.google.com/specimen/Roboto

## Licensing

- Ultimate Oldschool PC Font Pack: CC-BY-SA-4.0 — free to redistribute with credit; share-alike on modifications.
- Roboto: Apache 2.0 — free to redistribute.
