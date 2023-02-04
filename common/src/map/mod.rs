

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum CountryKind {
    OrangeStar,
    BlueMoon,
    GreenEarth,
    YellowComet,
    BlackHole,
    GreySky,
    BrownDesert,
    AmberBlaze,
    JadeSun,
    PinkCosmos,
    TealGalaxy,
    PurpleLightning,
    AcidRain,
    WhiteNove,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum TileKind {
    Plain,
    Mountain,
    Forest,
    River,
    Road,
    Bridge,
    Sea,
    Shoal,
    Reef,
    City,
    Base,
    Airport,
    Harbour,
    HeadQuarters,
    Pipe,
    Silo,
    CommunicationsTower,
    Laboratory,
}

impl TileKind {
    pub fn hides_units(&self) -> bool {
        match self {
            TileKind::Forest => true,
            TileKind::Reef => true,
            _ => false,
        }
    }
}