pub mod item_structs;

use item_structs::*;

use mob_structs::*;

use prop_structs::*;
mod mob_structs;
mod prop_structs;
mod rawmaster;
mod spawn_table_structs;
mod loot_structs;
use loot_structs::*;

pub use rawmaster::*;
use serde::{Deserialize};
use std::sync::Mutex;
use lazy_static::lazy_static;
use item_structs::Item;
use crate::raws::spawn_table_structs::SpawnTableEntry;

//TODO initialize this in a function along with modded raws
rltk::embedded_resource!(RAW_FILE, "../../raws/spawns.json");

lazy_static! {
    pub static ref RAWS : Mutex<RawMaster> = Mutex::new(RawMaster::empty());
}

#[derive(Deserialize, Debug)]
pub struct Raws {
    pub items : Vec<Item>,
    pub mobs : Vec<Mob>,
    pub props : Vec<Prop>,
    pub spawn_table : Vec<SpawnTableEntry>,
    pub loot_tables : Vec<LootTable>
}

pub fn load_raws(path:String) {
    rltk::link_resource!(RAW_FILE, path);

    // Retrieve the raw data as an array of u8 (8-bit unsigned chars)
    let raw_data = rltk::embedding::EMBED
        .lock()
        .get_resource(path.to_string())
        .unwrap();
    let raw_string = std::str::from_utf8(&raw_data).expect("Unable to convert to a valid UTF-8 string.");
    let decoder : Raws = serde_json::from_str(&raw_string).expect("Unable to parse JSON");

    RAWS.lock().unwrap().load(decoder);
}