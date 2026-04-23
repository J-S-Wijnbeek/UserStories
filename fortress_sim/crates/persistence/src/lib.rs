use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use world::TileMap;
use sim_core::{Dwarf, Item, JobQueue, SimTime};
use gameplay::Stockpile;

pub const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct SaveMeta {
    pub version: u32,
    pub seed: u64,
    pub tick: u64,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct WorldData {
    pub map: TileMap,
    pub dwarves: Vec<Dwarf>,
    pub items: Vec<Item>,
    pub job_queue: JobQueue,
    pub stockpiles: Vec<Stockpile>,
}

pub fn save(
    path: &str,
    seed: u64,
    time: &SimTime,
    map: &TileMap,
    dwarves: &[Dwarf],
    items: &[Item],
    job_queue: &JobQueue,
    stockpiles: &[Stockpile],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(path)?;

    let meta = SaveMeta {
        version: CURRENT_VERSION,
        seed,
        tick: time.tick,
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let meta_path = Path::new(path).join("meta.json");
    fs::write(meta_path, serde_json::to_string_pretty(&meta)?)?;

    let world_data = WorldData {
        map: map.clone(),
        dwarves: dwarves.to_vec(),
        items: items.to_vec(),
        job_queue: job_queue.clone(),
        stockpiles: stockpiles.to_vec(),
    };

    let bin_path = Path::new(path).join("world.bin");
    fs::write(bin_path, bincode::serialize(&world_data)?)?;

    Ok(())
}

pub fn load(path: &str) -> Result<(SaveMeta, WorldData), Box<dyn std::error::Error>> {
    let meta_path = Path::new(path).join("meta.json");
    let meta: SaveMeta = serde_json::from_str(&fs::read_to_string(meta_path)?)?;

    if meta.version != CURRENT_VERSION {
        return Err(format!("Incompatible save version: {} (expected {})", meta.version, CURRENT_VERSION).into());
    }

    let bin_path = Path::new(path).join("world.bin");
    let world_data: WorldData = bincode::deserialize(&fs::read(bin_path)?)?;

    Ok((meta, world_data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use world::WorldGen;
    use sim_core::{SimTime, JobQueue};

    #[test]
    fn test_save_load_roundtrip() {
        let map = WorldGen::generate(12345);
        let time = SimTime::new();
        let dwarves = vec![];
        let items = vec![];
        let job_queue = JobQueue::new();
        let stockpiles = vec![];

        let path = "./test_saves/roundtrip_test";
        save(path, 12345, &time, &map, &dwarves, &items, &job_queue, &stockpiles).unwrap();
        let (meta, data) = load(path).unwrap();
        assert_eq!(meta.version, CURRENT_VERSION);
        assert_eq!(meta.seed, 12345);
        assert_eq!(data.map.width, 64);
        assert_eq!(data.map.height, 64);

        // cleanup
        std::fs::remove_dir_all("./test_saves").ok();
    }
}
