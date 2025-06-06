use std::cmp::PartialEq;
use rltk::{RGB, RandomNumberGenerator };
use specs::prelude::*;
use super::{Player, Renderable, Name, Position, Viewshed, Rect, SerializeMe, random_table::RandomTable, HungerClock, HungerState, Map, raws::*, Attributes, Attribute, Skill, Skills, Pools, Pool, LightSource, Initiative, Faction};
use specs::saveload::{MarkedBuilder, SimpleMarker};
use std::collections::HashMap;
use std::mem;
use crate::gamesystem::{attr_bonus, mana_at_level, player_hp_at_level};
use crate::map::{tile_walkable, TileType};
use crate::map::TileType::Floor;
use crate::map_builders::walkable;

/// Spawns the player and returns his entity object.
pub fn player(ecs : &mut World, player_x : i32, player_y : i32) -> Entity {
    let mut skills = Skills{ skills: HashMap::new() };
    skills.skills.insert(Skill::Melee, 1);
    skills.skills.insert(Skill::Defense, 1);
    skills.skills.insert(Skill::Magic, 1);

    let player = ecs
        .create_entity()
        .with(Position { x: player_x, y: player_y })
        .with(Renderable {
            glyph: rltk::to_cp437('@'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 0
        })
        .with(Player{})
        .with(Viewshed{ visible_tiles : Vec::new(), range: 8, dirty: true })
        .with(Name{name: "Player".to_string() })
        .with(HungerClock{ state: HungerState::WellFed, duration: 20 })
        .with(Attributes{
            might: Attribute{ base: 11, modifiers: 0, bonus: attr_bonus(11) },
            fitness: Attribute{ base: 11, modifiers: 0, bonus: attr_bonus(11) },
            quickness: Attribute{ base: 11, modifiers: 0, bonus: attr_bonus(11) },
            intelligence: Attribute{ base: 11, modifiers: 0, bonus: attr_bonus(11) },
        })
        .with(skills)
        .with(Pools{
            hit_points : Pool{
                current: player_hp_at_level(11, 1),
                max: player_hp_at_level(11, 1)
            },
            mana: Pool{
                current: mana_at_level(11, 1),
                max: mana_at_level(11, 1)
            },
            xp: 0,
            level: 1
        })
        // player's "torch" to navigate underground unlit areas
        //TODO add game mechanics that upgrade/change/unlock this
        .with(LightSource{ color: rltk::RGB::from_f32(1.0, 1.0, 0.5), range: 8 })
        // player should go first
        .with(Initiative{current: 0})
        .with(Faction{name : "Player".to_string() })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();

    // Starting equipment
    // MAKE SURE THIS ONLY HAPPENS ON NEW GAME
    spawn_named_entity(&RAWS.lock().unwrap(), ecs, "Rusted Longsword -1", SpawnType::Equipped{by : player});
    spawn_named_entity(&RAWS.lock().unwrap(), ecs, "Dried Sausage", SpawnType::Carried{by : player} );
    spawn_named_entity(&RAWS.lock().unwrap(), ecs, "Beer", SpawnType::Carried{by : player});
    spawn_named_entity(&RAWS.lock().unwrap(), ecs, "Stained Tunic", SpawnType::Equipped{by : player});
    spawn_named_entity(&RAWS.lock().unwrap(), ecs, "Torn Trousers", SpawnType::Equipped{by : player});
    spawn_named_entity(&RAWS.lock().unwrap(), ecs, "Old Boots", SpawnType::Equipped{by : player});

    player
}

const MAX_MONSTERS : i32 = 4;

fn room_table(map_depth: i32) -> RandomTable {
    get_spawn_table_for_depth(&RAWS.lock().unwrap(), map_depth)
}

/// Fills a room with stuff!
pub fn spawn_room(map: &Map, rng: &mut RandomNumberGenerator, room : &Rect, map_depth: i32, spawn_list : &mut Vec<(usize, String)>) {
    let mut possible_targets : Vec<usize> = Vec::new();
    { // Borrow scope - to keep access to the map separated
        for y in room.y1 + 1 .. room.y2 {
            for x in room.x1 + 1 .. room.x2 {
                let idx = map.xy_idx(x, y);
                if map.tiles[idx] == TileType::Floor {
                    possible_targets.push(idx);
                }
            }
        }
    }

    spawn_region(map, rng, &possible_targets, map_depth, spawn_list);
}

/// Fills a region with stuff!
pub fn spawn_region(_map: &Map, rng: &mut RandomNumberGenerator, area : &[usize], map_depth: i32, spawn_list : &mut Vec<(usize, String)>) {
    let spawn_table = room_table(map_depth);
    let mut spawn_points : HashMap<usize, String> = HashMap::new();
    let mut areas : Vec<usize> = Vec::from(area);

    // Scope to keep the borrow checker happy
    {
        let num_spawns = i32::min(areas.len() as i32, rng.roll_dice(1, MAX_MONSTERS + 3) + (map_depth - 1) - 3);
        if num_spawns == 0 { return; }

        for _i in 0 .. num_spawns {
            let array_index = if areas.len() == 1 { 0usize } else { (rng.roll_dice(1, areas.len() as i32)-1) as usize };

            let map_idx = areas[array_index];
            let to_spawn = spawn_table.roll(rng);
            
            if _map.tiles[map_idx] == Floor {
                spawn_points.insert(map_idx, to_spawn);
            }else{
                rltk::console::log("Unable to spawn inside Walls in the spawn region")
            }
            
            areas.remove(array_index);
        }
    }

    // Actually spawn the monsters
    for spawn in spawn_points.iter() {
        spawn_list.push((*spawn.0, spawn.1.to_string()));
    }
}

/// Spawns a named entity (name in tuple.1) at the location in (tuple.0)
pub fn spawn_entity(ecs: &mut World, spawn : &(&usize, &String)) {
    let map = ecs.fetch::<Map>();
    let width = map.width as usize;
    let x = (*spawn.0 % width) as i32;
    let y = (*spawn.0 / width) as i32;

    if tile_walkable(map.tiles[map.xy_idx(x,y)]) {
        mem::drop(map);
        let spawn_result = spawn_named_entity(&RAWS.lock().unwrap(), ecs, &spawn.1, SpawnType::AtPosition{ x, y});
        if spawn_result.is_some() {
            return;
        }

        rltk::console::log(format!("WARNING: We don't know how to spawn [{}]!", spawn.1));
    }else {
        rltk::console::log(format!("Unable to spawn objects in Walls at ({},{})", x, y));
    }
}