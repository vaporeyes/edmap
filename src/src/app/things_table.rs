// ABOUTME: Static lookup tables for DOOM thing types — radius (for bounding-box
// ABOUTME: rendering) and category (for the Things filter dialog).

/// Index of each category in the thing_filter array. Order matches
/// THING_CATEGORY_LABELS in dialog.rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    PlayerStart = 0,
    Teleport = 1,
    Monster = 2,
    Weapon = 3,
    Ammo = 4,
    Health = 5,
    Powerup = 6,
    Key = 7,
    Obstacle = 8,
    Light = 9,
    Decoration = 10,
}

impl Category {
    pub fn idx(self) -> usize { self as usize }
}

/// Look up a thing type's category. Defaults to Decoration if unknown so
/// custom mod things don't disappear when filters are active.
pub fn category_of(thing_type: u16) -> Category {
    match thing_type {
        // Player Starts
        1 | 2 | 3 | 4 | 11 => Category::PlayerStart,
        // Teleports
        14 => Category::Teleport,
        // Monsters (DOOM I + II + Heretic core set)
        9 | 65 | 66 | 67 | 68 | 69 | 71 | 84
        | 3001 | 3002 | 3003 | 3004 | 3005 | 3006 => Category::Monster,
        7 | 16 | 58 | 64 => Category::Monster,
        // Weapons
        2001 | 2002 | 2003 | 2004 | 2005 | 2006 => Category::Weapon,
        // Ammunition
        2007 | 2008 | 2010 | 2046 | 2047 | 2048 | 2049 | 17 => Category::Ammo,
        // Health & Armor
        2011 | 2012 | 2013 | 2014 | 2015 | 2018 | 2019 => Category::Health,
        // Powerups
        2022 | 2023 | 2024 | 2025 | 2026 | 2045 | 83 => Category::Powerup,
        // Keys
        5 | 6 | 13 | 38 | 39 | 40 => Category::Key,
        // Obstacles
        25..=28 | 29..=37 | 41..=43 | 44..=48 | 54..=57 | 70 | 72..=82 => Category::Obstacle,
        // Light Sources
        2028 | 34 | 35 => Category::Light,
        // Everything else → Decoration
        _ => Category::Decoration,
    }
}

/// DOOM thing radius lookup. Defaults to 16 (player size) for unknown types.
/// Sourced from Matt Fell's Unofficial DOOM Specs.
pub fn radius_of(thing_type: u16) -> i32 {
    match thing_type {
        1 | 2 | 3 | 4 => 16,             // Player starts
        11 => 16,                         // DM start
        14 => 16,                         // teleport destination
        // Monsters
        3001 => 20,                       // Imp
        3002 => 30,                       // Demon
        58 => 30,                         // Spectre
        3003 => 24,                       // Baron of Hell
        3004 => 20,                       // Zombieman
        9 => 20,                          // Sergeant
        3005 => 31,                       // Cacodemon
        3006 => 16,                       // Lost Soul
        65 => 20,                         // Chaingunner
        66 => 20,                         // Revenant
        67 => 48,                         // Mancubus
        68 => 64,                         // Arachnotron
        69 => 24,                         // Hell Knight
        71 => 31,                         // Pain Elemental
        7 => 128,                         // Spider Mastermind
        16 => 40,                         // Cyberdemon
        84 => 20,                         // SS Nazi
        64 => 20,                         // Arch-Vile
        // Default: player-size for items, decorations.
        _ => 16,
    }
}

/// Whether the radius lookup is meaningful for this thing (player+monsters
/// have meaningful collision; pickups don't).
pub fn renders_collision_box(thing_type: u16) -> bool {
    matches!(category_of(thing_type), Category::Monster | Category::PlayerStart)
}

/// Candidate DOOM sprite lump names for a thing type, in priority order. The
/// first one the texture bank can resolve wins. Monsters use `<PREFIX>A1`
/// (frame A, angle 1 = facing south) since rotated sprites exist; pickups use
/// `<PREFIX>A0` (single all-angle frame). Returns an empty slice for unknown
/// types so no preview pops up rather than showing the wrong image.
pub fn sprite_candidates(thing_type: u16) -> &'static [&'static str] {
    match thing_type {
        // Player starts / DM start
        1 | 2 | 3 | 4 | 11 => &["PLAYA1"],
        // Teleport destination
        14 => &["TFOGA0"],
        // Monsters
        3004 => &["POSSA1"], // Zombieman
        9    => &["SPOSA1"], // Sergeant (shotgun guy)
        65   => &["CPOSA1"], // Chaingunner
        3001 => &["TROOA1"], // Imp
        3002 => &["SARGA1"], // Demon
        58   => &["SARGA1"], // Spectre (same sprite, drawn translucent in-game)
        3006 => &["SKULA1"], // Lost Soul
        3005 => &["HEADA1"], // Cacodemon
        69   => &["BOS2A1"], // Hell Knight
        3003 => &["BOSSA1"], // Baron of Hell
        68   => &["BSPIA1"], // Arachnotron
        71   => &["PAINA1"], // Pain Elemental
        66   => &["SKELA1"], // Revenant
        67   => &["FATTA1"], // Mancubus
        64   => &["VILEA1"], // Arch-Vile
        7    => &["SPIDA1"], // Spider Mastermind
        16   => &["CYBRA1"], // Cyberdemon
        84   => &["SSWVA1"], // SS Nazi
        72   => &["KEENA0"], // Commander Keen
        88   => &["BBRNA0"], // Romero head (boss target)
        // Weapons
        2001 => &["SHOTA0"],
        82   => &["SGN2A0"],
        2002 => &["MGUNA0"],
        2003 => &["LAUNA0"],
        2004 => &["PLASA0"],
        2005 => &["CSAWA0"],
        2006 => &["BFUGA0"],
        // Ammo
        2007 => &["CLIPA0"],
        2008 => &["SHELA0"],
        2010 => &["ROCKA0"],
        2046 => &["BROKA0"],
        2047 => &["CELLA0"],
        2048 => &["AMMOA0"],
        2049 => &["SBOXA0"],
        17   => &["CELPA0"],
        // Health & armor
        2011 => &["STIMA0"],
        2012 => &["MEDIA0"],
        2013 => &["SOULA0"],
        2014 => &["BON1A0"],
        2015 => &["BON2A0"],
        2018 => &["ARM1A0"],
        2019 => &["ARM2A0"],
        // Powerups
        2022 => &["PINVA0"],
        2023 => &["PSTRA0"],
        2024 => &["PINSA0"],
        2025 => &["SUITA0"],
        2026 => &["PMAPA0"],
        2045 => &["PVISA0"],
        83   => &["MEGAA0"],
        8    => &["BPAKA0"],
        // Keys
        5    => &["BKEYB0", "BKEYA0"],
        40   => &["BSKUB0", "BSKUA0"],
        13   => &["RKEYB0", "RKEYA0"],
        38   => &["RSKUB0", "RSKUA0"],
        6    => &["YKEYB0", "YKEYA0"],
        39   => &["YSKUB0", "YSKUA0"],
        // Common obstacles / decoration
        2035 => &["BAR1A0"], // Exploding barrel
        70   => &["FCANA0"], // Burning barrel
        43   => &["TRE1A0"],
        54   => &["TRE2A0"],
        2028 => &["COLUA0"], // Floor lamp
        34   => &["CANDA0"],
        35   => &["CBRAA0"], // Candelabra
        44   => &["TBLUA0"],
        45   => &["TGRNA0"],
        46   => &["TREDA0"],
        55   => &["SMBTA0"],
        56   => &["SMGTA0"],
        57   => &["SMRTA0"],
        47   => &["SMITA0"],
        48   => &["ELECA0"],
        _ => &[],
    }
}
