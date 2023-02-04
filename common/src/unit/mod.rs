

/**
 * All of the possible units that can be used in a game.
 */
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum UnitKind {
    AntiAir,
    Apc,
    Artillery,
    BattleCopter,
    BattleShip,
    BlackBoat,
    BlackBomb,
    Bomber,
    Carrier,
    Cruiser,
    Fighter,
    Infantry,
    Lander,
    MediumTank,
    Mech,
    MegaTank,
    Missile,
    NeoTank,
    PipeRunner,
    Recon,
    Rocket,
    Stealth,
    Submarine,
    TransportCopter,
    Tank,
}

impl UnitKind {
    pub fn vision(&self) -> u8 {
        match self {
            UnitKind::AntiAir => 2,
            UnitKind::Apc => 1,
            UnitKind::Artillery => 1,
            UnitKind::BattleCopter => 3,
            UnitKind::BattleShip => 2,
            UnitKind::BlackBoat => 1,
            UnitKind::BlackBomb => 1,
            UnitKind::Bomber => 2,
            UnitKind::Carrier => 4,
            UnitKind::Cruiser => 3,
            UnitKind::Fighter => 2,
            UnitKind::Infantry => 2,
            UnitKind::Lander => 1,
            UnitKind::MediumTank => 1,
            UnitKind::Mech => 2,
            UnitKind::MegaTank => 1,
            UnitKind::Missile => 5,
            UnitKind::NeoTank => 1,
            UnitKind::PipeRunner => 4,
            UnitKind::Recon => 5,
            UnitKind::Rocket => 1,
            UnitKind::Stealth => 4,
            UnitKind::Submarine => 5,
            UnitKind::TransportCopter => 2,
            UnitKind::Tank => 3,
        }
    }
}
