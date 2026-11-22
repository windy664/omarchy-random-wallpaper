//! What may end up on the wall.
//!
//! Both lists are deliberately conservative: a picture that is not clearly a
//! landscape or a city is not applied. Showing nothing is better than showing
//! a picture of a coffee cup.

/// Substrings that mark a Bing slug as a landscape or a city.
///
/// Matched against the lowercased slug, so the entries are lowercase.
const SCENIC: &[&str] = &[
    // mountains and high ground
    "mountain",
    "peak",
    "ridge",
    "alps",
    "himalaya",
    "volcano",
    "glacier",
    "iceberg",
    "fjord",
    "cliff",
    "canyon",
    "valley",
    "dune",
    "desert",
    "hill",
    "countryside",
    "meadow",
    "field",
    "farm",
    "alpine",
    "badlands",
    "prairie",
    "trail",
    "footpath",
    "plateau",
    "mesa",
    "butte",
    "hoodoo",
    "tundra",
    "steppe",
    "savanna",
    // woods and plants
    "forest",
    "wood",
    "jungle",
    "tree",
    "cypress",
    "oak",
    "blossom",
    "flower",
    "autumn",
    "spring",
    "summer",
    "winter",
    "snow",
    "frost",
    "woodland",
    "moor",
    "pine",
    "birch",
    "maple",
    "vineyard",
    "wheat",
    "lavender",
    "olive",
    "orchard",
    // water
    "lake",
    "loch",
    "river",
    "creek",
    "waterfall",
    "pond",
    "marsh",
    "wetland",
    "sea",
    "ocean",
    "coast",
    "shore",
    "beach",
    "bay",
    "cove",
    "island",
    "isle",
    "harbor",
    "harbour",
    "lighthouse",
    "wave",
    "reef",
    "falls",
    "cascade",
    "brook",
    "stream",
    "lagoon",
    "tarn",
    "cape",
    "peninsula",
    "swamp",
    "bayou",
    "keys",
    "coastline",
    "seaside",
    "oasis",
    "waterhole",
    // sky and weather
    "sky",
    "sunset",
    "sunrise",
    "dusk",
    "dawn",
    "cloud",
    "storm",
    "mist",
    "fog",
    "aurora",
    "northernlights",
    "star",
    "milkyway",
    "galaxy",
    "eclipse",
    "moon",
    "night",
    "geothermal",
    "geyser",
    // cities
    "city",
    "skyline",
    "skyscraper",
    "downtown",
    "metropolis",
    "street",
    "avenue",
    "boulevard",
    "bridge",
    "tower",
    "castle",
    "palace",
    "cathedral",
    "church",
    "temple",
    "shrine",
    "fortress",
    "monastery",
    "village",
    "town",
    "oldtown",
    "marina",
    "pier",
    "walk",
    "park",
    "garden",
    "gorge",
    "stones",
    "rocks",
    "crane",
    "heron",
    "swan",
    "flamingo",
    "safari",
    "windmill",
    "canal",
    "plaza",
    "square",
    "viaduct",
    "aqueduct",
    "courtyard",
    "pagoda",
    "archipelago",
    "atoll",
    "karst",
    "bison",
    "elk",
    "moose",
    "deer",
    "wolf",
    "fox",
    "eagle",
    "owl",
    "wildlife",
    "panorama",
];

/// Substrings that rule a Bing slug out.
///
/// These win over the whitelist: a picture of a Christmas market is a
/// Christmas market even when the slug also says "square".
const BING_REJECTED: &[&str] = &[
    "halloween",
    "christmas",
    "xmas",
    "easter",
    "valentine",
    "thanksgiving",
    "newyear",
    "carnival",
    "fest",
    "party",
    "library",
    "museum",
    "gallery",
    "coffee",
    "food",
    "cake",
    "dessert",
    "laptop",
    "workspace",
    "desk",
    "interior",
    "studio",
    "shop",
    "market",
    "car",
    "bicycle",
    "motorcycle",
    "airplane",
    "spacecraft",
    "robot",
    "abstract",
    "pattern",
    "typography",
    "illustration",
    "collage",
    "sculpture",
];

/// Substrings that rule a Wikimedia Commons title out.
///
/// The Commons categories hold a lot more than landscapes, so the titles are
/// filtered against this list as well.
const COMMONS_REJECTED: &[&str] = &[
    // people and portraits
    "portrait ",
    "portrait of",
    "person",
    "statue",
    "self-portrait",
    "nude",
    "doll",
    // artwork and documents
    "painting",
    "engraving",
    "poster",
    "cover of",
    "book",
    "map of",
    "flag of",
    "coat of arms",
    "logo",
    "stamp",
    "coin",
    "medal",
    "graffiti",
    "drawing",
    "artwork",
    "render",
    "concept art",
    // interiors
    "interior",
    "library",
    "museum",
    "gallery",
    // religious and memorial architecture
    "cathedral",
    "church",
    "mosque",
    "chapel",
    "abbey",
    "basilica",
    "altar",
    "organ",
    "stained glass",
    "cemetery",
    "tomb",
    "ruins",
    // civic and royal buildings
    "fountain",
    "town hall",
    "university",
    "monastery",
    "palace",
    "castle",
    // fantasy
    "monster",
    "alien",
    "ghost",
    "myth",
    "legend",
    "dragon",
    "angel",
];

/// Whether a Bing slug describes a picture worth showing.
pub fn is_scenic(slug: &str) -> bool {
    let lower = slug.to_lowercase();

    if BING_REJECTED.iter().any(|keyword| lower.contains(keyword)) {
        return false;
    }

    SCENIC.iter().any(|keyword| lower.contains(keyword))
}

/// Whether a Wikimedia Commons title rules the picture out.
pub fn is_rejected(title: &str) -> bool {
    let lower = title.to_lowercase();
    COMMONS_REJECTED
        .iter()
        .any(|keyword| lower.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mountains_and_cities_pass() {
        assert!(is_scenic("OHR.AlpineBraies_EN-US6636326357"));
        assert!(is_scenic("OHR.GilaCliff_ROW5960011422"));
        assert!(is_scenic("OHR.CitySkyline_ZH-CN123"));
    }

    #[test]
    fn anything_else_does_not() {
        assert!(!is_scenic("OHR.HalloweenPumpkins_EN-US1"));
        assert!(!is_scenic("OHR.KnowledgeSculpture_EN-US1"));
        assert!(!is_scenic("OHR.SomethingUnheard"));
    }

    #[test]
    fn titles_are_filtered() {
        assert!(is_rejected("Portrait of a woman"));
        assert!(is_rejected("Castle Neuschwanstein"));
        assert!(!is_rejected("Blue hour over the harbour"));
    }
}
