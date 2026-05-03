// ABOUTME: Static categorized data for the Thing-type and LineDef-action pickers.
// ABOUTME: Tables sourced from the public DOOM thing/linedef-action references.

/// One row in a categorized picker tree.
pub struct PickerEntry {
    pub code: u16,
    pub label: &'static str,
}

pub struct PickerCategory {
    pub label: &'static str,
    pub entries: &'static [PickerEntry],
}

/// Find the human label for a numeric code in a picker table; returns the
/// numeric form ("(123)") if not found.
pub fn label_for(table: &[PickerCategory], code: u16) -> String {
    for cat in table {
        for entry in cat.entries {
            if entry.code == code {
                return format!("{} — {}", entry.code, entry.label);
            }
        }
    }
    format!("{code}")
}

// ---------------- Thing types ----------------

const PLAYER_STARTS: &[PickerEntry] = &[
    PickerEntry { code: 1, label: "Player 1 start" },
    PickerEntry { code: 2, label: "Player 2 start" },
    PickerEntry { code: 3, label: "Player 3 start" },
    PickerEntry { code: 4, label: "Player 4 start" },
    PickerEntry { code: 11, label: "Deathmatch start" },
];

const TELEPORTS: &[PickerEntry] = &[
    PickerEntry { code: 14, label: "Teleport destination" },
];

const MONSTERS: &[PickerEntry] = &[
    PickerEntry { code: 3001, label: "Imp" },
    PickerEntry { code: 3002, label: "Demon" },
    PickerEntry { code: 58,   label: "Spectre" },
    PickerEntry { code: 3003, label: "Baron of Hell" },
    PickerEntry { code: 3004, label: "Zombieman" },
    PickerEntry { code: 9,    label: "Sergeant (Shotgun guy)" },
    PickerEntry { code: 3005, label: "Cacodemon" },
    PickerEntry { code: 3006, label: "Lost Soul" },
    PickerEntry { code: 65,   label: "Chaingunner" },
    PickerEntry { code: 66,   label: "Revenant" },
    PickerEntry { code: 67,   label: "Mancubus" },
    PickerEntry { code: 68,   label: "Arachnotron" },
    PickerEntry { code: 69,   label: "Hell Knight" },
    PickerEntry { code: 71,   label: "Pain Elemental" },
    PickerEntry { code: 7,    label: "Spider Mastermind" },
    PickerEntry { code: 16,   label: "Cyberdemon" },
    PickerEntry { code: 84,   label: "SS Nazi (Wolfenstein)" },
    PickerEntry { code: 64,   label: "Arch-Vile" },
];

const WEAPONS: &[PickerEntry] = &[
    PickerEntry { code: 2001, label: "Shotgun" },
    PickerEntry { code: 82,   label: "Super Shotgun" },
    PickerEntry { code: 2002, label: "Chaingun" },
    PickerEntry { code: 2003, label: "Rocket Launcher" },
    PickerEntry { code: 2004, label: "Plasma Gun" },
    PickerEntry { code: 2005, label: "Chainsaw" },
    PickerEntry { code: 2006, label: "BFG 9000" },
];

const AMMO: &[PickerEntry] = &[
    PickerEntry { code: 2007, label: "Clip" },
    PickerEntry { code: 2048, label: "Box of bullets" },
    PickerEntry { code: 2008, label: "4 Shotgun shells" },
    PickerEntry { code: 2049, label: "Box of shells" },
    PickerEntry { code: 2010, label: "Rocket" },
    PickerEntry { code: 2046, label: "Box of rockets" },
    PickerEntry { code: 2047, label: "Cell charge" },
    PickerEntry { code: 17,   label: "Cell pack" },
    PickerEntry { code: 8,    label: "Backpack" },
];

const HEALTH: &[PickerEntry] = &[
    PickerEntry { code: 2011, label: "Stim Pack" },
    PickerEntry { code: 2012, label: "Medikit" },
    PickerEntry { code: 2014, label: "Health bonus" },
    PickerEntry { code: 2015, label: "Armor bonus" },
    PickerEntry { code: 2018, label: "Green armor" },
    PickerEntry { code: 2019, label: "Blue armor" },
    PickerEntry { code: 2013, label: "Soul Sphere (+100)" },
    PickerEntry { code: 83,   label: "MegaSphere" },
];

const POWERUPS: &[PickerEntry] = &[
    PickerEntry { code: 2022, label: "Invulnerability" },
    PickerEntry { code: 2023, label: "Berserk" },
    PickerEntry { code: 2024, label: "Partial invisibility" },
    PickerEntry { code: 2025, label: "Radiation suit" },
    PickerEntry { code: 2026, label: "Computer area map" },
    PickerEntry { code: 2045, label: "Light amplification visor" },
];

const KEYS: &[PickerEntry] = &[
    PickerEntry { code: 5,  label: "Blue keycard" },
    PickerEntry { code: 40, label: "Blue skull key" },
    PickerEntry { code: 13, label: "Red keycard" },
    PickerEntry { code: 38, label: "Red skull key" },
    PickerEntry { code: 6,  label: "Yellow keycard" },
    PickerEntry { code: 39, label: "Yellow skull key" },
];

const OBSTACLES: &[PickerEntry] = &[
    PickerEntry { code: 2035, label: "Barrel" },
    PickerEntry { code: 47,   label: "Stalagmite" },
    PickerEntry { code: 43,   label: "Burning tree" },
    PickerEntry { code: 25,   label: "Skewered impaled corpse" },
    PickerEntry { code: 26,   label: "Pile of corpses" },
    PickerEntry { code: 27,   label: "Skewered impaled corpse #2" },
    PickerEntry { code: 28,   label: "Five skulls (shish kebob)" },
    PickerEntry { code: 30,   label: "Tall green pillar" },
    PickerEntry { code: 31,   label: "Short green pillar" },
    PickerEntry { code: 32,   label: "Tall red pillar" },
    PickerEntry { code: 33,   label: "Short red pillar" },
    PickerEntry { code: 41,   label: "Evil eye" },
    PickerEntry { code: 42,   label: "Floating skull rock" },
    PickerEntry { code: 70,   label: "Burning barrel" },
    PickerEntry { code: 73,   label: "Hanging victim, twitching" },
];

const LIGHTS: &[PickerEntry] = &[
    PickerEntry { code: 34,   label: "Candle" },
    PickerEntry { code: 35,   label: "Candelabra" },
    PickerEntry { code: 44,   label: "Tall blue firestick" },
    PickerEntry { code: 45,   label: "Tall green firestick" },
    PickerEntry { code: 46,   label: "Tall red firestick" },
    PickerEntry { code: 55,   label: "Short blue firestick" },
    PickerEntry { code: 56,   label: "Short green firestick" },
    PickerEntry { code: 57,   label: "Short red firestick" },
    PickerEntry { code: 2028, label: "Floor lamp" },
    PickerEntry { code: 85,   label: "Tall techno column lamp" },
    PickerEntry { code: 86,   label: "Short techno lamp" },
];

const DECORATIONS: &[PickerEntry] = &[
    PickerEntry { code: 10, label: "Bloody mess #1" },
    PickerEntry { code: 12, label: "Bloody mess #2" },
    PickerEntry { code: 15, label: "Dead player" },
    PickerEntry { code: 18, label: "Dead former human" },
    PickerEntry { code: 19, label: "Dead sergeant" },
    PickerEntry { code: 20, label: "Dead imp" },
    PickerEntry { code: 21, label: "Dead demon" },
    PickerEntry { code: 22, label: "Dead cacodemon" },
    PickerEntry { code: 23, label: "Dead lost soul (invisible)" },
    PickerEntry { code: 24, label: "Pool of blood" },
    PickerEntry { code: 79, label: "Pool of blood #2" },
    PickerEntry { code: 80, label: "Pool of blood #3" },
    PickerEntry { code: 81, label: "Pool of brains" },
];

pub const THING_TYPES: &[PickerCategory] = &[
    PickerCategory { label: "Player Starts", entries: PLAYER_STARTS },
    PickerCategory { label: "Teleports",     entries: TELEPORTS },
    PickerCategory { label: "Monsters",      entries: MONSTERS },
    PickerCategory { label: "Weapons",       entries: WEAPONS },
    PickerCategory { label: "Ammunition",    entries: AMMO },
    PickerCategory { label: "Health & Armor",entries: HEALTH },
    PickerCategory { label: "Powerups",      entries: POWERUPS },
    PickerCategory { label: "Keys",          entries: KEYS },
    PickerCategory { label: "Obstacles",     entries: OBSTACLES },
    PickerCategory { label: "Light Sources", entries: LIGHTS },
    PickerCategory { label: "Decoration",    entries: DECORATIONS },
];

// ---------------- LineDef actions ----------------

const ACTIONS_NORMAL: &[PickerEntry] = &[
    PickerEntry { code: 0, label: "Normal (no action)" },
];

const ACTIONS_DOORS: &[PickerEntry] = &[
    PickerEntry { code: 1,   label: "DR Door (also monsters)" },
    PickerEntry { code: 26,  label: "DR Door (Blue key)" },
    PickerEntry { code: 27,  label: "DR Door (Yellow key)" },
    PickerEntry { code: 28,  label: "DR Door (Red key)" },
    PickerEntry { code: 117, label: "DR Door fast" },
    PickerEntry { code: 31,  label: "D1 Door open stay" },
    PickerEntry { code: 118, label: "D1 Door open stay fast" },
    PickerEntry { code: 32,  label: "D1 Door open stay (Blue key)" },
    PickerEntry { code: 33,  label: "D1 Door open stay (Red key)" },
    PickerEntry { code: 34,  label: "D1 Door open stay (Yellow key)" },
    PickerEntry { code: 2,   label: "W1 Door open stay" },
    PickerEntry { code: 86,  label: "WR Door open stay" },
    PickerEntry { code: 105, label: "WR Door open wait close fast" },
];

const ACTIONS_LIFTS: &[PickerEntry] = &[
    PickerEntry { code: 62,  label: "S1 Lift (switch, once)" },
    PickerEntry { code: 88,  label: "WR Lift (walk repeat)" },
    PickerEntry { code: 121, label: "W1 Lift fast (walk once)" },
    PickerEntry { code: 123, label: "SR Lift fast (switch repeat)" },
];

const ACTIONS_TELEPORTS: &[PickerEntry] = &[
    PickerEntry { code: 39, label: "W1 Teleport" },
    PickerEntry { code: 97, label: "WR Teleport" },
    PickerEntry { code: 125, label: "W1 Teleport (monsters only)" },
    PickerEntry { code: 126, label: "WR Teleport (monsters only)" },
];

const ACTIONS_FLOORS: &[PickerEntry] = &[
    PickerEntry { code: 19, label: "W1 Floor lower to highest neighbor" },
    PickerEntry { code: 5,  label: "W1 Floor raise to lowest ceiling" },
    PickerEntry { code: 14, label: "S1 Floor raise 32 fast" },
    PickerEntry { code: 60, label: "SR Floor lower to lowest" },
];

const ACTIONS_CEILINGS: &[PickerEntry] = &[
    PickerEntry { code: 40, label: "W1 Ceiling raise to highest" },
    PickerEntry { code: 44, label: "W1 Ceiling lower to floor + 8" },
];

const ACTIONS_CRUSHERS: &[PickerEntry] = &[
    PickerEntry { code: 6,   label: "W1 Crusher fast" },
    PickerEntry { code: 25,  label: "W1 Crusher slow" },
    PickerEntry { code: 73,  label: "WR Crusher slow" },
    PickerEntry { code: 77,  label: "WR Crusher fast" },
];

const ACTIONS_STAIRS: &[PickerEntry] = &[
    PickerEntry { code: 8,   label: "W1 Stairs (8 step)" },
    PickerEntry { code: 100, label: "W1 Stairs fast (16 step)" },
];

const ACTIONS_LIGHT: &[PickerEntry] = &[
    PickerEntry { code: 35, label: "W1 Light to 35 (darkest)" },
    PickerEntry { code: 12, label: "W1 Light to highest neighbor" },
    PickerEntry { code: 13, label: "W1 Light to 255" },
    PickerEntry { code: 17, label: "W1 Light blink 1 second" },
];

const ACTIONS_EXIT: &[PickerEntry] = &[
    PickerEntry { code: 11,  label: "S1 Exit normal" },
    PickerEntry { code: 51,  label: "S1 Exit secret" },
    PickerEntry { code: 52,  label: "W1 Exit normal" },
    PickerEntry { code: 124, label: "W1 Exit secret" },
];

const ACTIONS_SCROLL: &[PickerEntry] = &[
    PickerEntry { code: 48, label: "Scrolling wall (left)" },
    PickerEntry { code: 85, label: "Scrolling wall (right)" },
];

pub const LINEDEF_ACTIONS: &[PickerCategory] = &[
    PickerCategory { label: "Normal",     entries: ACTIONS_NORMAL },
    PickerCategory { label: "Doors",      entries: ACTIONS_DOORS },
    PickerCategory { label: "Lifts",      entries: ACTIONS_LIFTS },
    PickerCategory { label: "Teleports",  entries: ACTIONS_TELEPORTS },
    PickerCategory { label: "Floors",     entries: ACTIONS_FLOORS },
    PickerCategory { label: "Ceilings",   entries: ACTIONS_CEILINGS },
    PickerCategory { label: "Crushers",   entries: ACTIONS_CRUSHERS },
    PickerCategory { label: "Stairs",     entries: ACTIONS_STAIRS },
    PickerCategory { label: "Light",      entries: ACTIONS_LIGHT },
    PickerCategory { label: "Exit",       entries: ACTIONS_EXIT },
    PickerCategory { label: "Scroll",     entries: ACTIONS_SCROLL },
];
