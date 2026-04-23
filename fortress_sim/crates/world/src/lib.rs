use serde::{Deserialize, Serialize};
use rand::{SeedableRng, Rng};
use rand::rngs::StdRng;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum TileKind {
    Solid,
    Wall,
    Floor,
    Open,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Stone,
    Ore,
    Wood,
    Food,
    None,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum DesignationKind {
    Mine,
    Haul,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Tile {
    pub kind: TileKind,
    pub material: Material,
    pub designated: Option<DesignationKind>,
}

impl Default for Tile {
    fn default() -> Self {
        Tile {
            kind: TileKind::Solid,
            material: Material::None,
            designated: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TileMap {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Tile>,
}

impl TileMap {
    pub fn new() -> Self {
        let width = 64u32;
        let height = 64u32;
        let tiles = vec![Tile::default(); (width * height) as usize];
        TileMap { width, height, tiles }
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x < self.width && y < self.height {
            Some((y * self.width + x) as usize)
        } else {
            None
        }
    }

    pub fn get(&self, x: u32, y: u32) -> Option<&Tile> {
        self.index(x, y).map(|i| &self.tiles[i])
    }

    pub fn get_mut(&mut self, x: u32, y: u32) -> Option<&mut Tile> {
        self.index(x, y).map(|i| &mut self.tiles[i])
    }

    pub fn set(&mut self, x: u32, y: u32, tile: Tile) {
        if let Some(i) = self.index(x, y) {
            self.tiles[i] = tile;
        }
    }

    pub fn is_walkable(&self, x: u32, y: u32) -> bool {
        match self.get(x, y) {
            Some(tile) => matches!(tile.kind, TileKind::Floor | TileKind::Open),
            None => false,
        }
    }
}

impl Default for TileMap {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WorldGen;

impl WorldGen {
    pub fn generate(seed: u64) -> TileMap {
        let mut map = TileMap::new();
        let mut rng = StdRng::seed_from_u64(seed);

        // Place solid border walls
        for x in 0..64u32 {
            map.set(x, 0, Tile { kind: TileKind::Wall, material: Material::Stone, designated: None });
            map.set(x, 63, Tile { kind: TileKind::Wall, material: Material::Stone, designated: None });
        }
        for y in 0..64u32 {
            map.set(0, y, Tile { kind: TileKind::Wall, material: Material::Stone, designated: None });
            map.set(63, y, Tile { kind: TileKind::Wall, material: Material::Stone, designated: None });
        }

        // Carve a 20x20 open area in the center: x in [22,41], y in [22,41]
        for y in 22..=41u32 {
            for x in 22..=41u32 {
                map.set(x, y, Tile { kind: TileKind::Floor, material: Material::None, designated: None });
            }
        }

        // Scatter stone/wood items using rng
        for _ in 0..30 {
            let x = rng.gen_range(22..=41u32);
            let y = rng.gen_range(22..=41u32);
            let material = if rng.gen_bool(0.5) { Material::Stone } else { Material::Wood };
            if let Some(tile) = map.get_mut(x, y) {
                tile.material = material;
            }
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tilemap_get_set() {
        let mut map = TileMap::new();
        let tile = Tile { kind: TileKind::Floor, material: Material::Stone, designated: None };
        map.set(5, 5, tile.clone());
        let got = map.get(5, 5).unwrap();
        assert_eq!(got.kind, TileKind::Floor);
        assert_eq!(got.material, Material::Stone);
    }

    #[test]
    fn test_is_walkable() {
        let mut map = TileMap::new();
        assert!(!map.is_walkable(5, 5)); // Solid by default
        map.set(5, 5, Tile { kind: TileKind::Floor, material: Material::None, designated: None });
        assert!(map.is_walkable(5, 5));
    }
}
