// ABOUTME: Friendly map titles for the standard DOOM/DOOM II/Heretic IWADs.
// ABOUTME: EdMap originally shipped these baked-in; we look up by uppercase lump name.

/// Returns the canonical title for a known IWAD map lump name (e.g. "MAP17" -> "Tenements").
/// Returns None for unknown / PWAD-custom maps.
///
/// E1M1-style names are ambiguous between DOOM and Heretic. We prefer the DOOM
/// title and fall back to Heretic only when DOOM has no entry for that slot.
pub fn title_for(lump: &str) -> Option<&'static str> {
    let upper = lump.to_ascii_uppercase();
    if let Some(t) = doom2(&upper) {
        return Some(t);
    }
    if let Some(t) = doom(&upper) {
        return Some(t);
    }
    heretic(&upper)
}

fn doom2(name: &str) -> Option<&'static str> {
    Some(match name {
        "MAP01" => "Entryway",
        "MAP02" => "Underhalls",
        "MAP03" => "The Gantlet",
        "MAP04" => "The Focus",
        "MAP05" => "The Waste Tunnels",
        "MAP06" => "The Crusher",
        "MAP07" => "Dead Simple",
        "MAP08" => "Tricks and Traps",
        "MAP09" => "The Pit",
        "MAP10" => "Refueling Base",
        "MAP11" => "'O' of Destruction!",
        "MAP12" => "The Factory",
        "MAP13" => "Downtown",
        "MAP14" => "The Inmost Dens",
        "MAP15" => "Industrial Zone",
        "MAP16" => "Suburbs",
        "MAP17" => "Tenements",
        "MAP18" => "The Courtyard",
        "MAP19" => "The Citadel",
        "MAP20" => "Gotcha!",
        "MAP21" => "Nirvana",
        "MAP22" => "The Catacombs",
        "MAP23" => "Barrels o' Fun",
        "MAP24" => "The Chasm",
        "MAP25" => "Bloodfalls",
        "MAP26" => "The Abandoned Mines",
        "MAP27" => "Monster Condo",
        "MAP28" => "The Spirit World",
        "MAP29" => "The Living End",
        "MAP30" => "Icon of Sin",
        "MAP31" => "Wolfenstein",
        "MAP32" => "Grosse",
        _ => return None,
    })
}

fn doom(name: &str) -> Option<&'static str> {
    Some(match name {
        "E1M1" => "Hangar",
        "E1M2" => "Nuclear Plant",
        "E1M3" => "Toxin Refinery",
        "E1M4" => "Command Control",
        "E1M5" => "Phobos Lab",
        "E1M6" => "Central Processing",
        "E1M7" => "Computer Station",
        "E1M8" => "Phobos Anomaly",
        "E1M9" => "Military Base",
        "E2M1" => "Deimos Anomaly",
        "E2M2" => "Containment Area",
        "E2M3" => "Refinery",
        "E2M4" => "Deimos Lab",
        "E2M5" => "Command Center",
        "E2M6" => "Halls of the Damned",
        "E2M7" => "Spawning Vats",
        "E2M8" => "Tower of Babel",
        "E2M9" => "Fortress of Mystery",
        "E3M1" => "Hell Keep",
        "E3M2" => "Slough of Despair",
        "E3M3" => "Pandemonium",
        "E3M4" => "House of Pain",
        "E3M5" => "Unholy Cathedral",
        "E3M6" => "Mt. Erebus",
        "E3M7" => "Limbo",
        "E3M8" => "Dis",
        "E3M9" => "Warrens",
        "E4M1" => "Hell Beneath",
        "E4M2" => "Perfect Hatred",
        "E4M3" => "Sever the Wicked",
        "E4M4" => "Unruly Evil",
        "E4M5" => "They Will Repent",
        "E4M6" => "Against Thee Wickedly",
        "E4M7" => "And Hell Followed",
        "E4M8" => "Unto the Cruel",
        "E4M9" => "Fear",
        _ => return None,
    })
}

fn heretic(name: &str) -> Option<&'static str> {
    Some(match name {
        "E1M1" => "The Docks",
        "E1M2" => "The Dungeons",
        "E1M3" => "The Gatehouse",
        "E1M4" => "The Guard Tower",
        "E1M5" => "The Citadel",
        "E1M6" => "The Cathedral",
        "E1M7" => "The Crypts",
        "E1M8" => "Hell's Maw",
        "E1M9" => "The Graveyard",
        "E2M1" => "The Crater",
        "E2M2" => "The Lava Pits",
        "E2M3" => "The River of Fire",
        "E2M4" => "The Ice Grotto",
        "E2M5" => "The Catacombs",
        "E2M6" => "The Labyrinth",
        "E2M7" => "The Great Hall",
        "E2M8" => "The Portals of Chaos",
        "E2M9" => "The Glacier",
        "E3M1" => "The Storehouse",
        "E3M2" => "The Cesspool",
        "E3M3" => "The Confluence",
        "E3M4" => "The Azure Fortress",
        "E3M5" => "The Ophidian Lair",
        "E3M6" => "The Halls of Fear",
        "E3M7" => "The Chasm",
        "E3M8" => "D'Sparil's Keep",
        "E3M9" => "The Aquifer",
        "E4M1" => "Catafalque",
        "E4M2" => "Blockhouse",
        "E4M3" => "Ambulatory",
        "E4M4" => "Sepulcher",
        "E4M5" => "Great Stair",
        "E4M6" => "Halls of the Apostate",
        "E4M7" => "Ramparts of Perdition",
        "E4M8" => "Shattered Bridge",
        "E4M9" => "Mausoleum",
        "E5M1" => "Ochre Cliffs",
        "E5M2" => "Rapids",
        "E5M3" => "Quay",
        "E5M4" => "Courtyard",
        "E5M5" => "Hydratyr",
        "E5M6" => "Colonnade",
        "E5M7" => "Foetid Manse",
        "E5M8" => "Field of Judgment",
        "E5M9" => "Skein of D'Sparil",
        _ => return None,
    })
}
