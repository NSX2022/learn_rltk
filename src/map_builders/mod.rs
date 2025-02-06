use super::{Map, Rect, TileType, Position, spawner};
mod simple_map;
use simple_map::SimpleMapBuilder;
mod common;
mod bsp_dungeon;
mod bsp_interior;
mod cellular_automata;

use common::*;
use specs::prelude::*;
use crate::map_builders::bsp_dungeon::BspDungeonBuilder;
use crate::map_builders::bsp_interior::BspInteriorBuilder;
use crate::map_builders::cellular_automata::CellularAutomataBuilder;

const MIN_ROOM_SIZE: i32 = 3;


pub trait MapBuilder {
    fn build_map(&mut self);
    fn spawn_entities(&mut self, ecs : &mut World);
    fn get_map(&self) -> Map;
    fn get_starting_position(&self) -> Position;
    fn get_snapshot_history(&self) -> Vec<Map>;
    fn take_snapshot(&mut self);
}

pub fn random_builder(new_depth: i32) -> Box<dyn MapBuilder> {
    /*let mut rng = rltk::RandomNumberGenerator::new();
    let builder = rng.roll_dice(1, 3);
    match builder {
        1 => Box::new(BspDungeonBuilder::new(new_depth)),
        2 => Box::new(BspInteriorBuilder::new(new_depth)),
        _ => Box::new(SimpleMapBuilder::new(new_depth))
    }*/
    Box::new(CellularAutomataBuilder::new(new_depth))
}
