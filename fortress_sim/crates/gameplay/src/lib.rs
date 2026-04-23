use serde::{Deserialize, Serialize};
use world::{TileMap, Tile, TileKind, Material, DesignationKind};
use sim_core::{ItemId, ItemKind, Item};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stockpile {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub accepted: Vec<Material>,
    pub items: Vec<ItemId>,
}

impl Stockpile {
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    pub fn accepts(&self, mat: Material) -> bool {
        self.accepted.contains(&mat)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub inputs: Vec<(Material, u32)>,
    pub output_material: Material,
    pub output_kind: ItemKind,
    pub output_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workshop {
    pub id: u32,
    pub pos: (u32, u32),
    pub recipes: Vec<Recipe>,
}

pub struct GameplayState {
    pub stockpiles: Vec<Stockpile>,
    pub workshops: Vec<Workshop>,
}

impl GameplayState {
    pub fn new() -> Self {
        GameplayState {
            stockpiles: Vec::new(),
            workshops: Vec::new(),
        }
    }

    pub fn standard_recipes() -> Vec<Recipe> {
        vec![
            Recipe {
                inputs: vec![(Material::Wood, 2)],
                output_material: Material::Wood,
                output_kind: ItemKind::Plank,
                output_count: 4,
            },
            Recipe {
                inputs: vec![(Material::Stone, 3)],
                output_material: Material::Stone,
                output_kind: ItemKind::Block,
                output_count: 4,
            },
            Recipe {
                inputs: vec![(Material::Food, 2)],
                output_material: Material::Food,
                output_kind: ItemKind::Meal,
                output_count: 1,
            },
        ]
    }

    pub fn process_mine_designation(
        map: &mut TileMap,
        x: u32,
        y: u32,
        items: &mut Vec<Item>,
        next_item_id: &mut u32,
    ) {
        let should_mine = map.get(x, y)
            .map(|t| t.designated == Some(DesignationKind::Mine))
            .unwrap_or(false);

        if should_mine {
            let mat = map.get(x, y).map(|t| t.material).unwrap_or(Material::Stone);
            map.set(x, y, Tile {
                kind: TileKind::Floor,
                material: Material::Stone,
                designated: None,
            });
            let item_id = sim_core::ItemId(*next_item_id);
            *next_item_id += 1;
            items.push(Item {
                id: item_id,
                pos: (x, y),
                material: mat,
                kind: ItemKind::Rock,
            });
        }
    }
}

impl Default for GameplayState {
    fn default() -> Self {
        Self::new()
    }
}
