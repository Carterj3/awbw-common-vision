use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use map::{CountryKind, TileKind};

use officer::{OfficerKind, PowerKind};
use unit::UnitKind;

pub mod map;
pub mod officer;
pub mod unit;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UnitState {
    /** Index into players of who owns the units. */
    player: usize,
    /** If true then only adjacent units can reveal it. */
    stealthed: bool,
    kind: UnitKind,
}

impl UnitState {
    fn new(player: usize, stealthed: bool, kind: UnitKind) -> UnitState {
        UnitState {
            player,
            stealthed,
            kind,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GameState {
    /** 1D Vec of the map starting from the top left. */
    map: Vec<TileKind>,
    /** The (width, height) of the map. */
    map_dimensions: (usize, usize),

    /** BTreeMap storing for at a given index in `map` what unit is stored
     * there. */
    units: BTreeMap<usize, UnitState>,

    players: Vec<(CountryKind, OfficerKind, PowerKind)>,
    teams: Vec<HashSet<usize>>,
}

impl GameState {
    /**
     * For a given location returns all of the tiles within a certain
     * distance of that tile.
     */
    fn neighbors(&self, location: usize, distance: usize) -> HashSet<usize> {
        use std::cmp::{max, min};

        let (width, height) = self.map_dimensions;
        let mut neighbors = HashSet::new();

        let (x, y) = (location % width, location / width);

        for w in
            x.saturating_sub(distance)..min(width, x.saturating_add(distance).saturating_add(1))
        {
            for h in y.saturating_sub(distance)
                ..min(height, y.saturating_add(distance).saturating_add(1))
            {
                let dx = max(w, x).saturating_sub(min(w, x));
                let dy = max(h, y).saturating_sub(min(h, y));

                if dy + dx <= distance {
                    neighbors.insert(h * width + w);
                }
            }
        }

        neighbors
    }

    /**
     * For a given location returns all of the tiles that are revealed by a
     * unit on that tile and which player (index) owns that unit.
     *
     * Returns None if no unit is on the tile.
     */
    // TODO: Player-owned buildings give vision of thier own tile
    fn vision_from_tiles(&self, location: usize) -> Option<(usize, HashSet<usize>)> {
        let Some(unit) = self.units.get(&location) else {
            return None;
        };

        let (owner_vision, forests_revealed) = match self.players.get(unit.player) {
            Some((_, OfficerKind::Sonja, PowerKind::Super)) => (2, true),
            Some((_, OfficerKind::Sonja, PowerKind::Normal)) => (2, true),
            Some((_, OfficerKind::Sonja, PowerKind::None)) => (1, false),
            _ => (0, false),
        };

        let vision_range = unit.kind.vision() + owner_vision;

        // Always reveal adjancent tiles (even if forest / stealthed)
        let mut revealed_locations = self.neighbors(location, 1);

        for neighbor in self.neighbors(location, vision_range as usize) {
            if self
                .units
                .get(&neighbor)
                .map(|unit_state| unit_state.stealthed)
                .unwrap_or(false)
            {
                // Distance Stealthed units are not revealed.
                continue;
            }

            if self
                .map
                .get(neighbor)
                .map(|tile| tile.hides_units())
                .unwrap_or(false)
                && !forests_revealed
            {
                // Typically units in forests are not revealed.
                continue;
            }

            revealed_locations.insert(neighbor);
        }

        Some((unit.player.clone(), revealed_locations))
    }

    /**
     * Returns a list containing for each team all of the locations that can
     * see the tile.
     */
    fn vision_for_units(&self, units: &BTreeMap<usize, UnitState>) -> Vec<Vec<HashSet<usize>>> {
        let player_to_team_map = {
            let mut map = HashMap::new();
            for (index, team) in self.teams.iter().enumerate() {
                for player in team.iter() {
                    map.insert(player.clone(), index);
                }
            }
            map
        };

        let mut empty_watchers = Vec::with_capacity(self.teams.len());
        for _ in 0..self.teams.len() {
            empty_watchers.push(HashSet::new());
        }

        let mut vision_data = Vec::with_capacity(self.map.len());
        for _ in 0..self.map.len() {
            vision_data.push(empty_watchers.clone());
        }

        for (location, _) in units.iter() {
            let Some((player, tiles)) = self.vision_from_tiles(location.clone()) else {
                continue;
           };

            let Some(team) = player_to_team_map.get(&player) else {
            continue;
           };

            for tile in tiles {
                vision_data
                    .get_mut(tile)
                    .expect("Tile was not in vision_state")
                    .get_mut(team.clone())
                    .expect("Team was not in watchers")
                    .insert(tile);
            }
        }

        vision_data
    }

    /**
     * Computes all of the tiles that are commonly visible to all players
     */
    pub fn common_vision(&self) -> HashSet<usize> {
        let mut visible_units = self.units.clone();
        let mut visible_tiles = self
            .map
            .iter()
            .enumerate()
            .map(|(index, _)| index)
            .collect::<HashSet<usize>>();

        for counter in 0..=self.units.len() {
            if counter == self.units.len() {
                // Algorithm is deterministic but avoid unbounded loops.
                return HashSet::new();
            }

            let mut vision_changed = false;

            for (location, teams) in self
                .vision_for_units(&visible_units)
                .into_iter()
                .enumerate()
            {
                let num_teams_with_vision =
                    teams.into_iter().filter(|units| !units.is_empty()).count();

                if num_teams_with_vision != self.teams.len() {
                    vision_changed = vision_changed
                        || visible_units.remove(&location).is_some()
                        || visible_tiles.remove(&location);
                }
            }

            if !vision_changed {
                break;
            }
        }

        visible_tiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn into_set(items: Vec<usize>) -> HashSet<usize> {
        items.into_iter().collect()
    }

    mod neighbors {
        use super::*;

        fn make_map(map_tile: TileKind, map_dimensions: (usize, usize)) -> GameState {
            let mut map = Vec::with_capacity(map_dimensions.0 * map_dimensions.1);
            for _ in 0..(map_dimensions.0 * map_dimensions.1) {
                map.push(map_tile.clone());
            }

            GameState {
                map,
                map_dimensions,
                units: BTreeMap::new(),
                players: Vec::new(),
                teams: Vec::new(),
            }
        }

        #[test]
        fn neighbors_1x1() {
            let game_state = make_map(TileKind::Sea, (1, 1));

            assert_eq!(into_set(vec![0]), game_state.neighbors(0, 1));
            assert_eq!(into_set(vec![0]), game_state.neighbors(0, 2));
            assert_eq!(into_set(vec![0]), game_state.neighbors(0, 3));

            // Perhaps shockingly, but an out of bounds index can have an in-bound neighbor
            assert_eq!(into_set(vec![0]), game_state.neighbors(1, 1));

            // However, if the out of bounds index is far enough it won't
            assert_eq!(into_set(vec![]), game_state.neighbors(100, 1));
        }

        #[test]
        fn neighbors_2x2() {
            let game_state = make_map(TileKind::Sea, (2, 2));

            assert_eq!(into_set(vec![0, 1, 2]), game_state.neighbors(0, 1));
            assert_eq!(into_set(vec![0, 1, 2, 3]), game_state.neighbors(0, 2));
            assert_eq!(into_set(vec![0, 1, 2, 3]), game_state.neighbors(0, 3));

            // Perhaps shockingly, but an out of bounds index can have an in-bound neighbor
            assert_eq!(into_set(vec![2]), game_state.neighbors(4, 1));

            // However, if the out of bounds index is far enough it won't
            assert_eq!(into_set(vec![]), game_state.neighbors(100, 1));
        }

        #[test]
        fn neighbors_3x3() {
            let game_state = make_map(TileKind::Sea, (3, 3));

            assert_eq!(into_set(vec![1, 3, 4, 5, 7]), game_state.neighbors(4, 1));
            assert_eq!(
                into_set(vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                game_state.neighbors(4, 2)
            );
            assert_eq!(
                into_set(vec![0, 1, 2, 3, 4, 5, 6, 7]),
                game_state.neighbors(0, 3)
            );
            assert_eq!(
                into_set(vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                game_state.neighbors(0, 4)
            );

            // Perhaps shockingly, but an out of bounds index can have an in-bound neighbor
            assert_eq!(into_set(vec![6]), game_state.neighbors(9, 1));

            // However, if the out of bounds index is far enough it won't
            assert_eq!(into_set(vec![]), game_state.neighbors(100, 1));
        }
    }

    mod vision_from_tiles {
        use super::*;

        #[test]
        pub fn simple_2x2() {
            let game_state = GameState {
                map: vec![
                    TileKind::HeadQuarters,
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::HeadQuarters,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Infantry)),
                    (3, UnitState::new(1, false, UnitKind::Infantry)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (CountryKind::OrangeStar, OfficerKind::Andy, PowerKind::None),
                    (CountryKind::BlueMoon, OfficerKind::Olaf, PowerKind::None),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(
                Some((0, into_set(vec![0, 1, 2, 3]))),
                game_state.vision_from_tiles(0)
            );
            assert_eq!(None, game_state.vision_from_tiles(1));
            assert_eq!(None, game_state.vision_from_tiles(2));
            assert_eq!(
                Some((1, into_set(vec![0, 1, 2, 3]))),
                game_state.vision_from_tiles(3)
            );
        }

        #[test]
        pub fn sonja_2x2() {
            let game_state = GameState {
                map: vec![
                    TileKind::HeadQuarters,
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::HeadQuarters,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Artillery)),
                    (3, UnitState::new(1, false, UnitKind::Artillery)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (CountryKind::OrangeStar, OfficerKind::Sonja, PowerKind::None),
                    (CountryKind::BlueMoon, OfficerKind::Sonja, PowerKind::None),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(
                Some((0, into_set(vec![0, 1, 2, 3]))),
                game_state.vision_from_tiles(0)
            );
            assert_eq!(None, game_state.vision_from_tiles(1));
            assert_eq!(None, game_state.vision_from_tiles(2));
            assert_eq!(
                Some((1, into_set(vec![0, 1, 2, 3]))),
                game_state.vision_from_tiles(3)
            );
        }

        #[test]
        pub fn sonja_2x2__forest__no_power() {
            let game_state = GameState {
                map: vec![
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Artillery)),
                    (3, UnitState::new(1, false, UnitKind::Artillery)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (CountryKind::OrangeStar, OfficerKind::Sonja, PowerKind::None),
                    (CountryKind::BlueMoon, OfficerKind::Sonja, PowerKind::None),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(
                Some((0, into_set(vec![0, 1, 2]))),
                game_state.vision_from_tiles(0)
            );
            assert_eq!(None, game_state.vision_from_tiles(1));
            assert_eq!(None, game_state.vision_from_tiles(2));
            assert_eq!(
                Some((1, into_set(vec![1, 2, 3]))),
                game_state.vision_from_tiles(3)
            );
        }

        #[test]
        pub fn sonja_2x2__forest__power() {
            let game_state = GameState {
                map: vec![
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Artillery)),
                    (3, UnitState::new(1, false, UnitKind::Artillery)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (
                        CountryKind::OrangeStar,
                        OfficerKind::Sonja,
                        PowerKind::Normal,
                    ),
                    (CountryKind::BlueMoon, OfficerKind::Sonja, PowerKind::Super),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(
                Some((0, into_set(vec![0, 1, 2, 3]))),
                game_state.vision_from_tiles(0)
            );
            assert_eq!(None, game_state.vision_from_tiles(1));
            assert_eq!(None, game_state.vision_from_tiles(2));
            assert_eq!(
                Some((1, into_set(vec![0, 1, 2, 3]))),
                game_state.vision_from_tiles(3)
            );
        }
    }

    mod common_vision {
        use super::*;

        #[test]
        pub fn simple_2x2_all() {
            let game_state = GameState {
                map: vec![
                    TileKind::HeadQuarters,
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::HeadQuarters,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Infantry)),
                    (3, UnitState::new(1, false, UnitKind::Infantry)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (CountryKind::OrangeStar, OfficerKind::Andy, PowerKind::None),
                    (CountryKind::BlueMoon, OfficerKind::Olaf, PowerKind::None),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(into_set(vec![0, 1, 2, 3]), game_state.common_vision());
        }

        #[test]
        pub fn simple_2x2_none() {
            let game_state = GameState {
                map: vec![
                    TileKind::HeadQuarters,
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::HeadQuarters,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Artillery)),
                    (3, UnitState::new(1, false, UnitKind::Artillery)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (CountryKind::OrangeStar, OfficerKind::Andy, PowerKind::None),
                    (CountryKind::BlueMoon, OfficerKind::Olaf, PowerKind::None),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(into_set(vec![]), game_state.common_vision());
        }

        #[test]
        pub fn sonja_2x2() {
            let game_state = GameState {
                map: vec![
                    TileKind::HeadQuarters,
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::HeadQuarters,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Artillery)),
                    (3, UnitState::new(1, false, UnitKind::Artillery)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (CountryKind::OrangeStar, OfficerKind::Sonja, PowerKind::None),
                    (CountryKind::BlueMoon, OfficerKind::Sonja, PowerKind::None),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(into_set(vec![0, 1, 2, 3]), game_state.common_vision());
        }

        #[test]
        pub fn sonja_2x2__forest__no_power() {
            let game_state = GameState {
                map: vec![
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Artillery)),
                    (3, UnitState::new(1, false, UnitKind::Artillery)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (CountryKind::OrangeStar, OfficerKind::Sonja, PowerKind::None),
                    (CountryKind::BlueMoon, OfficerKind::Sonja, PowerKind::None),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(into_set(vec![]), game_state.common_vision());
        }

        #[test]
        pub fn sonja_2x2__forest__power() {
            let game_state = GameState {
                map: vec![
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Artillery)),
                    (3, UnitState::new(1, false, UnitKind::Artillery)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (
                        CountryKind::OrangeStar,
                        OfficerKind::Sonja,
                        PowerKind::Normal,
                    ),
                    (CountryKind::BlueMoon, OfficerKind::Sonja, PowerKind::Super),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(into_set(vec![0, 1, 2, 3]), game_state.common_vision());
        }

        #[test]
        pub fn team_2x2__cycle__all() {
            let game_state = GameState {
                map: vec![
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                    TileKind::Forest,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Artillery)),
                    (1, UnitState::new(1, false, UnitKind::Artillery)),
                    (2, UnitState::new(2, false, UnitKind::Artillery)),
                    (3, UnitState::new(3, false, UnitKind::Artillery)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (CountryKind::OrangeStar, OfficerKind::Andy, PowerKind::None),
                    (CountryKind::BlueMoon, OfficerKind::Olaf, PowerKind::None),
                    (CountryKind::GreenEarth, OfficerKind::Drake, PowerKind::None),
                    (
                        CountryKind::YellowComet,
                        OfficerKind::Kanbei,
                        PowerKind::Super,
                    ),
                ],
                teams: vec![into_set(vec![0, 2]), into_set(vec![1, 3])],
            };

            assert_eq!(into_set(vec![0, 1, 2, 3]), game_state.common_vision());
        }

        #[test]
        pub fn team_3x3__recon() {
            let game_state = GameState {
                map: vec![
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::City,
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::Plain,
                    TileKind::Forest,
                ],
                map_dimensions: (2, 2),
                units: [
                    (0, UnitState::new(0, false, UnitKind::Artillery)),
                    (2, UnitState::new(1, false, UnitKind::Infantry)),
                    (8, UnitState::new(0, false, UnitKind::Recon)),
                ]
                .into_iter()
                .collect(),
                players: vec![
                    (CountryKind::OrangeStar, OfficerKind::Andy, PowerKind::None),
                    (CountryKind::BlueMoon, OfficerKind::Olaf, PowerKind::None),
                ],
                teams: vec![into_set(vec![0]), into_set(vec![1])],
            };

            assert_eq!(into_set(vec![]), game_state.common_vision());
        }
    }
}
