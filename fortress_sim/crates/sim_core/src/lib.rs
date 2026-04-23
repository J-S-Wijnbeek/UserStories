use serde::{Deserialize, Serialize};
use pathfinding::prelude::astar;
use rand::{SeedableRng, Rng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use world::{TileMap, Material, TileKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemKind {
    Rock,
    Log,
    Food,
    Drink,
    Plank,
    Block,
    Meal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub pos: (u32, u32),
    pub material: Material,
    pub kind: ItemKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dwarf {
    pub id: EntityId,
    pub name: String,
    pub pos: (u32, u32),
    pub hunger: f32,
    pub thirst: f32,
    pub sleep: f32,
    pub job: Option<JobId>,
    pub inventory: Vec<ItemId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimTime {
    pub tick: u64,
}

impl SimTime {
    pub fn new() -> Self {
        SimTime { tick: 0 }
    }

    pub fn advance(&mut self) {
        self.tick += 1;
    }

    pub fn day(&self) -> u64 {
        self.tick / 1000
    }

    pub fn ticks_per_day() -> u64 {
        1000
    }
}

impl Default for SimTime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JobKind {
    Dig { pos: (u32, u32) },
    Haul { item: ItemId, dest: (u32, u32) },
    Eat(ItemId),
    Drink(ItemId),
    Sleep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JobPriority {
    Critical = 0,
    Basic = 1,
    Low = 2,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub kind: JobKind,
    pub priority: JobPriority,
    pub claimed_by: Option<EntityId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobQueue {
    pub jobs: Vec<Job>,
    pub next_id: u32,
}

impl JobQueue {
    pub fn new() -> Self {
        JobQueue { jobs: Vec::new(), next_id: 0 }
    }

    pub fn push(&mut self, kind: JobKind, priority: JobPriority) -> JobId {
        let id = JobId(self.next_id);
        self.next_id += 1;
        self.jobs.push(Job { id, kind, priority, claimed_by: None });
        id
    }

    pub fn claim(&mut self, job_id: JobId, entity: EntityId) -> bool {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == job_id && j.claimed_by.is_none()) {
            job.claimed_by = Some(entity);
            true
        } else {
            false
        }
    }

    pub fn release(&mut self, job_id: JobId) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == job_id) {
            job.claimed_by = None;
        }
    }

    pub fn complete(&mut self, job_id: JobId) {
        self.jobs.retain(|j| j.id != job_id);
    }

    pub fn unclaimed(&self) -> Vec<&Job> {
        let mut unclaimed: Vec<&Job> = self.jobs.iter().filter(|j| j.claimed_by.is_none()).collect();
        unclaimed.sort_by_key(|j| j.priority);
        unclaimed
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub fn find_path(map: &TileMap, start: (u32, u32), goal: (u32, u32)) -> Option<Vec<(u32, u32)>> {
    if start == goal {
        return Some(vec![start]);
    }

    let result = astar(
        &start,
        |&(x, y)| {
            let mut neighbors = Vec::new();
            let directions: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
            for (dx, dy) in directions {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && ny >= 0 {
                    let nx = nx as u32;
                    let ny = ny as u32;
                    if map.is_walkable(nx, ny) || (nx == goal.0 && ny == goal.1) {
                        neighbors.push(((nx, ny), 1u32));
                    }
                }
            }
            neighbors
        },
        |&(x, y)| {
            let dx = (x as i32 - goal.0 as i32).unsigned_abs();
            let dy = (y as i32 - goal.1 as i32).unsigned_abs();
            dx + dy
        },
        |&pos| pos == goal,
    );

    result.map(|(path, _cost)| path)
}

pub struct SimState {
    pub dwarves: Vec<Dwarf>,
    pub items: Vec<Item>,
    pub job_queue: JobQueue,
    pub time: SimTime,
    pub paths: HashMap<EntityId, Vec<(u32, u32)>>,
    pub next_item_id: u32,
    rng: StdRng,
}

impl SimState {
    pub fn new(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let names = ["Urist", "Bomrek", "Fikod"];
        let dwarves = names.iter().enumerate().map(|(i, &name)| {
            Dwarf {
                id: EntityId(i as u32),
                name: name.to_string(),
                pos: (31 + i as u32, 31),
                hunger: rng.gen_range(0.0..0.3),
                thirst: rng.gen_range(0.0..0.3),
                sleep: rng.gen_range(0.0..0.3),
                job: None,
                inventory: Vec::new(),
            }
        }).collect();

        SimState {
            dwarves,
            items: Vec::new(),
            job_queue: JobQueue::new(),
            time: SimTime::new(),
            paths: HashMap::new(),
            next_item_id: 0,
            rng,
        }
    }

    pub fn step(&mut self, map: &mut TileMap) {
        self.time.advance();

        let dwarf_count = self.dwarves.len();
        for i in 0..dwarf_count {
            // Increase needs
            self.dwarves[i].hunger = (self.dwarves[i].hunger + 0.0001).min(1.0);
            self.dwarves[i].thirst = (self.dwarves[i].thirst + 0.00015).min(1.0);
            self.dwarves[i].sleep = (self.dwarves[i].sleep + 0.00008).min(1.0);

            let dwarf_id = self.dwarves[i].id;

            // Handle critical needs
            if self.dwarves[i].job.is_none() {
                let hunger = self.dwarves[i].hunger;
                let thirst = self.dwarves[i].thirst;
                let sleep = self.dwarves[i].sleep;

                if hunger > 0.8 {
                    let food_pos = self.items.iter()
                        .find(|item| matches!(item.kind, ItemKind::Food | ItemKind::Meal))
                        .map(|item| (item.id, item.pos));
                    if let Some((food_id, _pos)) = food_pos {
                        let job_id = self.job_queue.push(JobKind::Eat(food_id), JobPriority::Critical);
                        self.job_queue.claim(job_id, dwarf_id);
                        self.dwarves[i].job = Some(job_id);
                    }
                } else if thirst > 0.8 {
                    let drink_pos = self.items.iter()
                        .find(|item| matches!(item.kind, ItemKind::Drink))
                        .map(|item| (item.id, item.pos));
                    if let Some((drink_id, _pos)) = drink_pos {
                        let job_id = self.job_queue.push(JobKind::Drink(drink_id), JobPriority::Critical);
                        self.job_queue.claim(job_id, dwarf_id);
                        self.dwarves[i].job = Some(job_id);
                    }
                } else if sleep > 0.8 {
                    let job_id = self.job_queue.push(JobKind::Sleep, JobPriority::Critical);
                    self.job_queue.claim(job_id, dwarf_id);
                    self.dwarves[i].job = Some(job_id);
                }
            }

            // Claim unclaimed job if no current job
            if self.dwarves[i].job.is_none() {
                let unclaimed_id = self.job_queue.unclaimed().first().map(|j| j.id);
                if let Some(job_id) = unclaimed_id {
                    if self.job_queue.claim(job_id, dwarf_id) {
                        self.dwarves[i].job = Some(job_id);
                        self.paths.remove(&dwarf_id);
                    }
                }
            }

            // Advance toward job if one exists
            if let Some(job_id) = self.dwarves[i].job {
                let job_kind = self.job_queue.jobs.iter()
                    .find(|j| j.id == job_id)
                    .map(|j| j.kind.clone());

                if let Some(kind) = job_kind {
                    let dest = match &kind {
                        JobKind::Dig { pos } => Some(*pos),
                        JobKind::Haul { dest, .. } => Some(*dest),
                        JobKind::Eat(item_id) => {
                            self.items.iter().find(|it| it.id == *item_id).map(|it| it.pos)
                        }
                        JobKind::Drink(item_id) => {
                            self.items.iter().find(|it| it.id == *item_id).map(|it| it.pos)
                        }
                        JobKind::Sleep => Some(self.dwarves[i].pos),
                    };

                    if let Some(dest_pos) = dest {
                        let current_pos = self.dwarves[i].pos;

                        let at_dest = current_pos == dest_pos
                            || (current_pos.0 as i32 - dest_pos.0 as i32).abs() + (current_pos.1 as i32 - dest_pos.1 as i32).abs() <= 1;

                        if !at_dest {
                            let need_path = self.paths.get(&dwarf_id).map(|p| p.is_empty()).unwrap_or(true);
                            if need_path {
                                if let Some(path) = find_path(map, current_pos, dest_pos) {
                                    self.paths.insert(dwarf_id, path);
                                }
                            }

                            if let Some(path) = self.paths.get_mut(&dwarf_id) {
                                if path.first() == Some(&current_pos) {
                                    path.remove(0);
                                }
                                if let Some(&next) = path.first() {
                                    self.dwarves[i].pos = next;
                                    path.remove(0);
                                }
                            }
                        } else {
                            match kind {
                                JobKind::Dig { pos } => {
                                    let mat = map.get(pos.0, pos.1).map(|t| t.material).unwrap_or(Material::None);
                                    if let Some(tile) = map.get_mut(pos.0, pos.1) {
                                        tile.kind = TileKind::Floor;
                                        tile.designated = None;
                                    }
                                    let item_id = ItemId(self.next_item_id);
                                    self.next_item_id += 1;
                                    self.items.push(Item {
                                        id: item_id,
                                        pos,
                                        material: mat,
                                        kind: ItemKind::Rock,
                                    });
                                    self.job_queue.complete(job_id);
                                    self.dwarves[i].job = None;
                                    self.paths.remove(&dwarf_id);
                                }
                                JobKind::Sleep => {
                                    self.dwarves[i].sleep = (self.dwarves[i].sleep - 0.01).max(0.0);
                                    if self.dwarves[i].sleep < 0.1 {
                                        self.job_queue.complete(job_id);
                                        self.dwarves[i].job = None;
                                    }
                                }
                                JobKind::Eat(item_id) => {
                                    self.items.retain(|it| it.id != item_id);
                                    self.dwarves[i].hunger = 0.0;
                                    self.job_queue.complete(job_id);
                                    self.dwarves[i].job = None;
                                    self.paths.remove(&dwarf_id);
                                }
                                JobKind::Drink(item_id) => {
                                    self.items.retain(|it| it.id != item_id);
                                    self.dwarves[i].thirst = 0.0;
                                    self.job_queue.complete(job_id);
                                    self.dwarves[i].job = None;
                                    self.paths.remove(&dwarf_id);
                                }
                                JobKind::Haul { item: item_id, dest } => {
                                    if let Some(item) = self.items.iter_mut().find(|it| it.id == item_id) {
                                        item.pos = dest;
                                    }
                                    self.job_queue.complete(job_id);
                                    self.dwarves[i].job = None;
                                    self.paths.remove(&dwarf_id);
                                }
                            }
                        }
                    }
                } else {
                    self.dwarves[i].job = None;
                    self.paths.remove(&dwarf_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world::{TileMap, Tile, TileKind, Material};

    #[test]
    fn test_pathfinding_blocked() {
        let map = TileMap::new();
        let result = find_path(&map, (1, 1), (5, 5));
        assert!(result.is_none());
    }

    #[test]
    fn test_pathfinding_clear() {
        let mut map = TileMap::new();
        for x in 0..10 {
            map.set(x, 5, Tile { kind: TileKind::Floor, material: Material::None, designated: None });
        }
        let result = find_path(&map, (0, 5), (9, 5));
        assert!(result.is_some());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_determinism() {
        use world::WorldGen;
        let seed = 42u64;

        let mut map1 = WorldGen::generate(seed);
        let mut state1 = SimState::new(seed);
        for _ in 0..100 {
            state1.step(&mut map1);
        }

        let mut map2 = WorldGen::generate(seed);
        let mut state2 = SimState::new(seed);
        for _ in 0..100 {
            state2.step(&mut map2);
        }

        for (d1, d2) in state1.dwarves.iter().zip(state2.dwarves.iter()) {
            assert_eq!(d1.pos, d2.pos);
        }
    }
}
