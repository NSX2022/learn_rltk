use std::cell::Cell;
use super::{Map, Rect, TileType, Position, spawner, SHOW_MAPGEN_VISUALIZER, reject_map};
mod simple_map;
use simple_map::SimpleMapBuilder;
mod bsp_dungeon;
use bsp_dungeon::BspDungeonBuilder;
mod bsp_interior;
use bsp_interior::BspInteriorBuilder;
mod cellular_automata;
use cellular_automata::CellularAutomataBuilder;
mod drunkard;
use drunkard::*;
mod maze;
use maze::*;
mod dla;
use dla::*;
mod common;
use common::*;
mod voronoi;
use voronoi::*;
mod waveform_collapse;
use waveform_collapse::*;
mod prefab_builder;
mod room_based_spawner;
mod room_based_starting_position;
mod room_based_stairs;
mod area_starting_point;
mod cull_unreachable;
mod voronoi_spawning;
mod distant_exit;
mod room_exploder;
mod room_corner_rounding;
mod rooms_corridors_dogleg;
mod rooms_corridors_bsp;
mod room_sorter;
mod room_drawer;
mod room_corridors_nearest;
mod rooms_corridors_lines;
mod room_corridor_spawner;
mod door_placement;

use prefab_builder::*;
use specs::prelude::*;
use crate::map_builders::area_starting_point::{AreaStartingPosition, XStart, YStart};
use crate::map_builders::cull_unreachable::CullUnreachable;
use crate::map_builders::distant_exit::DistantExit;
use crate::map_builders::door_placement::DoorPlacement;
use crate::map_builders::prefab_builder::PrefabMode::RoomVaults;
use crate::map_builders::room_based_spawner::RoomBasedSpawner;
use crate::map_builders::room_based_stairs::RoomBasedStairs;
use crate::map_builders::room_based_starting_position::RoomBasedStartingPosition;
use crate::map_builders::room_corner_rounding::RoomCornerRounder;
use crate::map_builders::room_corridor_spawner::CorridorSpawner;
use crate::map_builders::room_corridors_nearest::NearestCorridors;
use crate::map_builders::room_drawer::RoomDrawer;
use crate::map_builders::room_exploder::RoomExploder;
use crate::map_builders::room_sorter::{RoomSort, RoomSorter};
use crate::map_builders::rooms_corridors_bsp::BspCorridors;
use crate::map_builders::rooms_corridors_dogleg::DoglegCorridors;
use crate::map_builders::rooms_corridors_lines::StraightLineCorridors;
use crate::map_builders::voronoi_spawning::VoronoiSpawning;

pub struct BuilderMap {
    pub spawn_list : Vec<(usize, String)>,
    pub map : Map,
    pub starting_position : Option<Position>,
    pub rooms: Option<Vec<Rect>>,
    pub corridors: Option<Vec<Vec<usize>>>,
    pub history : Vec<Map>,
    pub width: i32,
    pub height: i32
}

impl BuilderMap {
    fn take_snapshot(&mut self) {
        if SHOW_MAPGEN_VISUALIZER {
            let mut snapshot = self.map.clone();
            for v in snapshot.revealed_tiles.iter_mut() {
                *v = true;
            }
            self.history.push(snapshot);
        }
    }
}

pub struct BuilderChain {
    starter: Option<Box<dyn InitialMapBuilder>>,
    builders: Vec<Box<dyn MetaMapBuilder>>,
    pub build_data : BuilderMap
}

impl BuilderChain {
    pub fn new(new_depth : i32, width: i32, height: i32) -> BuilderChain {
        BuilderChain{
            starter: None,
            builders: Vec::new(),
            build_data : BuilderMap {
                spawn_list: Vec::new(),
                map: Map::new(new_depth, width, height),
                starting_position: None,
                rooms: None,
                corridors: None,
                history : Vec::new(),
                width,
                height
            }
        }
    }

    pub fn start_with(&mut self, starter : Box<dyn InitialMapBuilder>) {
        match self.starter {
            None => self.starter = Some(starter),
            Some(_) => panic!("You can only have one starting builder.")
        };
    }

    pub fn with(&mut self, metabuilder : Box<dyn MetaMapBuilder>) {
        self.builders.push(metabuilder);
    }

    pub fn build_map(&mut self, rng : &mut rltk::RandomNumberGenerator) {
        match &mut self.starter {
            None => panic!("Cannot run a map builder chain without a starting build system"),
            Some(starter) => {
                // Build the starting map
                starter.build_map(rng, &mut self.build_data);
            }
        }

        // Build additional layers in turn
        for metabuilder in self.builders.iter_mut() {
            metabuilder.build_map(rng, &mut self.build_data);
        }
    }

    pub fn spawn_entities(&mut self, ecs : &mut World) {
        for entity in self.build_data.spawn_list.iter() {
            spawner::spawn_entity(ecs, &(&entity.0, &entity.1));
        }
    }
}

pub struct EdgeBorderBuilder;

impl EdgeBorderBuilder {
    fn new() -> Box<dyn MetaMapBuilder> {
        Box::new(EdgeBorderBuilder)
    }
}

impl MetaMapBuilder for EdgeBorderBuilder {
    fn build_map(&mut self, _rng: &mut rltk::RandomNumberGenerator, build_data: &mut BuilderMap) {
        edge_border(&mut build_data.map);
    }
}

pub trait InitialMapBuilder {
    fn build_map(&mut self, rng: &mut rltk::RandomNumberGenerator, build_data : &mut BuilderMap);
}

pub trait MetaMapBuilder {
    fn build_map(&mut self, rng: &mut rltk::RandomNumberGenerator, build_data : &mut BuilderMap);
}

fn random_start_position(rng: &mut rltk::RandomNumberGenerator) -> (XStart, YStart) {
    let x;
    let xroll = rng.roll_dice(1, 3);
    match xroll {
        1 => x = XStart::LEFT,
        2 => x = XStart::CENTER,
        _ => x = XStart::RIGHT
    }

    let y;
    let yroll = rng.roll_dice(1, 3);
    match yroll {
        1 => y = YStart::BOTTOM,
        2 => y = YStart::CENTER,
        _ => y = YStart::TOP
    }

    (x, y)
}

fn random_room_builder(rng: &mut rltk::RandomNumberGenerator, builder : &mut BuilderChain) {
    let build_roll = rng.roll_dice(1, 3);
    match build_roll {
        1 => builder.start_with(SimpleMapBuilder::new()),
        2 => builder.start_with(BspDungeonBuilder::new()),
        _ => builder.start_with(BspInteriorBuilder::new())
    }

    // BSP Interior still makes holes in the walls
    if build_roll != 3 {
        // Sort by one of the 5 available algorithms
        let sort_roll = rng.roll_dice(1, 5);
        match sort_roll {
            1 => builder.with(RoomSorter::new(RoomSort::LEFTMOST)),
            2 => builder.with(RoomSorter::new(RoomSort::RIGHTMOST)),
            3 => builder.with(RoomSorter::new(RoomSort::TOPMOST)),
            4 => builder.with(RoomSorter::new(RoomSort::BOTTOMMOST)),
            _ => builder.with(RoomSorter::new(RoomSort::CENTRAL)),
        }

        builder.with(RoomDrawer::new());

        let corridor_roll = rng.roll_dice(1, 4);
        match corridor_roll {
            1 => builder.with(DoglegCorridors::new()),
            2 => builder.with(NearestCorridors::new()),
            3 => builder.with(StraightLineCorridors::new()),
            _ => builder.with(BspCorridors::new())
        }

        let cspawn_roll = rng.roll_dice(1, 2);
        if cspawn_roll == 1 {
            builder.with(CorridorSpawner::new());
        }

        let modifier_roll = rng.roll_dice(1, 6);
        match modifier_roll {
            1 => builder.with(RoomExploder::new()),
            2 => builder.with(RoomCornerRounder::new()),
            _ => {}
        }
    }

    let start_roll = rng.roll_dice(1, 2);
    match start_roll {
        1 => builder.with(RoomBasedStartingPosition::new()),
        _ => {
            let (start_x, start_y) = random_start_position(rng);
            builder.with(AreaStartingPosition::new(start_x, start_y));
        }
    }

    let exit_roll = rng.roll_dice(1, 2);
    match exit_roll {
        1 => builder.with(RoomBasedStairs::new()),
        _ => builder.with(DistantExit::new())
    }

    let spawn_roll = rng.roll_dice(1, 2);
    match spawn_roll {
        1 => builder.with(RoomBasedSpawner::new()),
        _ => builder.with(VoronoiSpawning::new())
    }
}

fn random_shape_builder(rng: &mut rltk::RandomNumberGenerator, builder : &mut BuilderChain) {
    let builder_roll = rng.roll_dice(1, 16);
    match builder_roll {
        1 => builder.start_with(CellularAutomataBuilder::new()),
        2 => builder.start_with(DrunkardsWalkBuilder::open_area()),
        3 => builder.start_with(DrunkardsWalkBuilder::open_halls()),
        4 => builder.start_with(DrunkardsWalkBuilder::winding_passages()),
        5 => builder.start_with(DrunkardsWalkBuilder::fat_passages()),
        6 => builder.start_with(DrunkardsWalkBuilder::fearful_symmetry()),
        7 => builder.start_with(MazeBuilder::new()),
        8 => builder.start_with(DLABuilder::walk_inwards()),
        9 => builder.start_with(DLABuilder::walk_outwards()),
        10 => builder.start_with(DLABuilder::central_attractor()),
        11 => builder.start_with(DLABuilder::insectoid()),
        12 => builder.start_with(VoronoiCellBuilder::pythagoras()),
        13 => builder.start_with(VoronoiCellBuilder::manhattan()),
        _ => builder.start_with(PrefabBuilder::constant(prefab_builder::prefab_levels::WFC_POPULATED)),
    }

    // Set the start to the center and cull
    builder.with(AreaStartingPosition::new(XStart::CENTER, YStart::CENTER));
    builder.with(CullUnreachable::new());

    // Now set the start to a random starting area
    let (start_x, start_y) = random_start_position(rng);
    builder.with(AreaStartingPosition::new(start_x, start_y));

    // Setup an exit and spawn mobs
    builder.with(VoronoiSpawning::new());
    builder.with(DistantExit::new());
}

pub fn random_builder(new_depth: i32, rng: &mut rltk::RandomNumberGenerator, width: i32, height: i32) -> BuilderChain {
    let mut builder = BuilderChain::new(new_depth, width, height);
    let type_roll = rng.roll_dice(1, 2);
    match type_roll {
        1 => random_room_builder(rng, &mut builder),
        _ => random_shape_builder(rng, &mut builder)
    }

    //TODO fix WFC making impossible maps and setting depth to 0
    /*
    if rng.roll_dice(1, 4)==1 {
        builder.with(WaveformCollapseBuilder::new());

        // Now set the start to a random starting area
        let (start_x, start_y) = random_start_position(rng);
        builder.with(AreaStartingPosition::new(start_x, start_y));

        // spawn mobs
        builder.with(VoronoiSpawning::new());
    }
     */
    

    if rng.roll_dice(1,100)==1 {
        //apply 1 iteration of cellular automata, leaving only the outline of rooms
        //this will most likely make an unplayable map
        builder.with(CellularAutomataBuilder::new());
        builder.with(VoronoiSpawning::new());
    }

    if rng.roll_dice(1, 30)==1 {
        builder.with(PrefabBuilder::sectional(prefab_builder::prefab_sections::UNDERGROUND_FORT));
    }
    
    if rng.roll_dice(1, 45) == 1{
        builder.with(PrefabBuilder::sectional(prefab_builder::prefab_sections::UNDERGROUND_FOUNTAIN));
    }

    builder.with(DoorPlacement::new());
    
    if rng.roll_dice(1,2)==1 {
        builder.with(PrefabBuilder::vaults());
    }

    //put a border of impassible tiles at the map's edge
    builder.with(EdgeBorderBuilder::new());
    
    builder.with(CullUnreachable::new());

    builder
}

pub fn edge_border(map: &mut Map) {
    let width = map.width;
    let height = map.height;

    // Iterate over the top and bottom rows
    for x in 0..width {
        let top_idx = map.xy_idx(x, 0); // Top row
        let bottom_idx = map.xy_idx(x, height - 1); // Bottom row
        map.tiles[top_idx] = TileType::Wall;
        map.tiles[bottom_idx] = TileType::Wall;
    }

    // Iterate over the left and right columns
    for y in 0..height {
        let left_idx = map.xy_idx(0, y); // Left column
        let right_idx = map.xy_idx(width - 1, y); // Right column
        map.tiles[left_idx] = TileType::Wall;
        map.tiles[right_idx] = TileType::Wall;
    }
}

pub fn stairs_present(ecs: &World) -> bool {
    let map = ecs.fetch::<Map>();
    let map_tile_count = (map.width * map.height) as usize;

    // Ensure that there is at least 1 stairway down
    for idx in 0..map_tile_count {
        if map.tiles[idx] == TileType::DownStairs {
            return true;
        }
    }
    false
}

pub fn walkable(ecs: &World, min_percent: i8) -> bool {
    let map = ecs.fetch::<Map>();
    let map_tile_count = (map.width * map.height) as usize;

    let mut num_walkable = 0;
    for idx in 0..map_tile_count {
        match map.tiles[idx] {
            TileType::DownStairs | TileType::Floor => num_walkable += 1,
            _ => {}
        }
    }

    let percent_walkable = (num_walkable as f32 / map_tile_count as f32) * 100.0;
    eprintln!("Walkable tiles: {}, Total tiles: {}, Percentage: {}", num_walkable, map_tile_count, percent_walkable);

    percent_walkable >= min_percent as f32
}
