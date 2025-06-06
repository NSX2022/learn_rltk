extern crate serde;

use std::sync::Mutex;
use lazy_static::lazy_static;
use rltk::{GameState, Point, Rltk};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};


mod components;
pub use components::*;
pub mod map;

#[path = "mods/directories.rs"] mod directories;

mod player;
use player::*;
mod rect;
pub use rect::Rect;
mod visibility_system;
use visibility_system::VisibilitySystem;
mod melee_combat_system;
use melee_combat_system::MeleeCombatSystem;
mod damage_system;
use damage_system::DamageSystem;
mod gui;
mod gamelog;
mod spawner;
mod inventory_system;
use inventory_system::{ItemCollectionSystem, ItemDropSystem, ItemRemoveSystem, ItemUseSystem};
use crate::directories::{config_exists, initialize, read_config};
use crate::map::Map;
use crate::map::map_indexing_system::MapIndexingSystem;
use crate::map_builders::{down_stairs_present, walkable};

use map::dungeon::MasterDungeonMap;
use crate::map::dungeon::level_transition;

pub mod saveload_system;
pub mod random_table;
pub mod particle_system;
pub mod hunger_system;
pub mod rex_assets;
pub mod trigger_system;
pub mod map_builders;
pub mod camera;
pub mod raws;
pub mod mods;
mod gamesystem;
mod lighting_system;
mod ai;

// Making it static so that the config can change it, apply Mutex for thread safety
// False by default
lazy_static! {
    static ref SHOW_MAPGEN_VISUALIZER: Mutex<bool> = Mutex::new(false);
}

// Making it static so that the config can change it, apply Mutex for thread safety
// False by default
lazy_static! {
    static ref SHOW_MAP_BORDER: Mutex<bool> = Mutex::new(false);
}

#[derive(PartialEq, Copy, Clone)]
pub enum RunState { 
    AwaitingInput,
    PreRun,
    Ticking,
    ShowInventory,
    ShowDropItem,
    ShowTargeting { range : i32, item : Entity},
    MainMenu { menu_selection : gui::MainMenuSelection },
    SaveGame,
    NextLevel,
    PreviousLevel,
    ShowRemoveItem,
    GameOver,
    MagicMapReveal { row : i32 },
    MapGeneration,
    ShowCheatMenu
}

pub struct State {
    pub ecs: World,
    mapgen_next_state : Option<RunState>,
    mapgen_history : Vec<Map>,
    mapgen_index : usize,
    mapgen_timer : f32
}

impl State {
    fn run_systems(&mut self) {
        let mut mapindex = MapIndexingSystem{};
        mapindex.run_now(&self.ecs);
        let mut vis = VisibilitySystem{};
        vis.run_now(&self.ecs);
        let mut initiative = ai::InitiativeSystem{};
        initiative.run_now(&self.ecs);
        let mut turnstatus = ai::TurnStatusSystem{};
        turnstatus.run_now(&self.ecs);
        let mut quipper = ai::QuipSystem{};
        quipper.run_now(&self.ecs);
        let mut mob = ai::MonsterAI{};
        mob.run_now(&self.ecs);
        let mut animal = ai::AnimalAI{};
        animal.run_now(&self.ecs);
        let mut bystander = ai::BystanderAI{};
        bystander.run_now(&self.ecs);
        let mut triggers = trigger_system::TriggerSystem{};
        triggers.run_now(&self.ecs);
        let mut melee = MeleeCombatSystem{};
        melee.run_now(&self.ecs);
        let mut damage = DamageSystem{};
        damage.run_now(&self.ecs);
        let mut pickup = ItemCollectionSystem{};
        pickup.run_now(&self.ecs);
        let mut itemuse = ItemUseSystem{};
        itemuse.run_now(&self.ecs);
        let mut drop_items = ItemDropSystem{};
        drop_items.run_now(&self.ecs);
        let mut item_remove = ItemRemoveSystem{};
        item_remove.run_now(&self.ecs);
        let mut hunger = hunger_system::HungerSystem{};
        hunger.run_now(&self.ecs);
        let mut particles = particle_system::ParticleSpawnSystem{};
        particles.run_now(&self.ecs);
        // Run lighting last
        let mut lighting = lighting_system::LightingSystem{};
        lighting.run_now(&self.ecs);

        self.ecs.maintain();
    }
}

impl GameState for State {
    fn tick(&mut self, ctx : &mut Rltk) {
        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
        }

        ctx.cls();
        particle_system::cull_dead_particles(&mut self.ecs, ctx);

        match newrunstate {
            RunState::MainMenu{..} => {}
            RunState::GameOver{..} => {}
            _ => {
                camera::render_camera(&self.ecs, ctx);
                gui::draw_ui(&self.ecs, ctx);
            }
        }

        match newrunstate {
            RunState::PreRun => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                newrunstate = player_input(self, ctx);
            }
            RunState::Ticking => {
                self.run_systems();
                self.ecs.maintain();
                match *self.ecs.fetch::<RunState>() {
                    RunState::AwaitingInput => newrunstate = RunState::AwaitingInput,
                    RunState::MagicMapReveal{ .. } => newrunstate = RunState::MagicMapReveal{ row: 0 },
                    _ => newrunstate = RunState::Ticking
                }
            }
            RunState::ShowInventory => {
                let result = gui::show_inventory(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let is_ranged = self.ecs.read_storage::<Ranged>();
                        let is_item_ranged = is_ranged.get(item_entity);
                        if let Some(is_item_ranged) = is_item_ranged {
                            newrunstate = RunState::ShowTargeting{ range: is_item_ranged.range, item: item_entity };
                        } else {
                            let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                            intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem{ item: item_entity, target: None }).expect("Unable to insert intent");
                            newrunstate = RunState::Ticking;
                        }
                    }
                }
            }
            RunState::ShowDropItem => {
                let result = gui::drop_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToDropItem{ item: item_entity }).expect("Unable to insert intent");
                        newrunstate = RunState::Ticking;
                    }
                }
            }
            RunState::ShowRemoveItem => {
                let result = gui::remove_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToRemoveItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToRemoveItem{ item: item_entity }).expect("Unable to insert intent");
                        newrunstate = RunState::Ticking;
                    }
                }
            }
            RunState::ShowTargeting{range, item} => {
                let result = gui::ranged_target(self, ctx, range);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem{ item, target: result.1 }).expect("Unable to insert intent");
                        newrunstate = RunState::Ticking;
                    }
                }
            }
            RunState::MainMenu{ .. } => {
                let result = gui::main_menu(self, ctx);
                match result {
                    gui::MainMenuResult::NoSelection{ selected } => newrunstate = RunState::MainMenu{ menu_selection: selected },
                    gui::MainMenuResult::Selected{ selected } => {
                        match selected {
                            gui::MainMenuSelection::NewGame => newrunstate = RunState::PreRun,
                            gui::MainMenuSelection::LoadGame => {
                                saveload_system::load_game(&mut self.ecs);
                                newrunstate = RunState::AwaitingInput;
                                saveload_system::delete_save();
                            }
                            gui::MainMenuSelection::Quit => { ::std::process::exit(0); }
                        }
                    }
                }
            }
            RunState::GameOver => {
                let result = gui::game_over(ctx);
                match result {
                    gui::GameOverResult::NoSelection => {}
                    gui::GameOverResult::QuitToMenu => {
                        self.game_over_cleanup();
                        newrunstate = RunState::MainMenu{ menu_selection: gui::MainMenuSelection::NewGame };
                    }
                }
            }
            RunState::SaveGame => {
                saveload_system::save_game(&mut self.ecs);
                newrunstate = RunState::MainMenu{ menu_selection : gui::MainMenuSelection::LoadGame };
            }
            RunState::NextLevel => {
                self.goto_level(1);
                self.mapgen_next_state = Some(RunState::PreRun);
                newrunstate = RunState::MapGeneration;
            }
            RunState::PreviousLevel => {
                self.goto_level(-1);
                self.mapgen_next_state = Some(RunState::PreRun);
                newrunstate = RunState::MapGeneration;
            }
            RunState::MagicMapReveal{row} => {
                let mut map = self.ecs.fetch_mut::<Map>();
                for x in 0..map.width {
                    let idx = map.xy_idx(x as i32,row);
                    map.revealed_tiles[idx] = true;
                }
                if row == map.height-1 {
                    newrunstate = RunState::Ticking;
                } else {
                    newrunstate = RunState::MagicMapReveal{ row: row+1 };
                }
            }
            RunState::MapGeneration => {
                if *SHOW_MAPGEN_VISUALIZER.lock().unwrap() {
                    ctx.cls();
                    if self.mapgen_index < self.mapgen_history.len() { camera::render_debug_map(&self.mapgen_history[self.mapgen_index], ctx); }

                    self.mapgen_timer += ctx.frame_time_ms;
                    if self.mapgen_timer > 200.0 {
                        self.mapgen_timer = 0.0;
                        self.mapgen_index += 1;
                        if self.mapgen_index >= self.mapgen_history.len() {
                            //self.mapgen_index -= 1;
                            newrunstate = self.mapgen_next_state.unwrap();
                        }
                    }
                } else {
                    newrunstate = self.mapgen_next_state.unwrap();
                }
            }
            RunState::ShowCheatMenu => {
                let result = gui::show_cheat_mode(self, ctx);
                match result {
                    gui::CheatMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::CheatMenuResult::NoResponse => {}
                    gui::CheatMenuResult::TeleportToExit => {
                        self.goto_level(1);
                        self.mapgen_next_state = Some(RunState::PreRun);
                        newrunstate = RunState::MapGeneration;
                    }
                }
            }
        }

        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }
        damage_system::delete_the_dead(&mut self.ecs);
    }
}

impl State {
    fn entities_to_remove_on_level_change(&mut self) -> Vec<Entity> {
        let entities = self.ecs.entities();
        let player = self.ecs.read_storage::<Player>();
        let backpack = self.ecs.read_storage::<InBackpack>();
        let player_entity = self.ecs.fetch::<Entity>();
        let equipped = self.ecs.read_storage::<Equipped>();

        let mut to_delete : Vec<Entity> = Vec::new();
        for entity in entities.join() {
            let mut should_delete = true;

            // Don't delete the player
            let p = player.get(entity);
            if let Some(_p) = p {
                should_delete = false;
            }

            // Don't delete the player's equipment
            let bp = backpack.get(entity);
            if let Some(bp) = bp {
                if bp.owner == *player_entity {
                    should_delete = false;
                }
            }

            let eq = equipped.get(entity);
            if let Some(eq) = eq {
                if eq.owner == *player_entity {
                    should_delete = false;
                }
            }

            if should_delete {
                to_delete.push(entity);
            }
        }

        to_delete
    }

    fn goto_level(&mut self, offset: i32) {
        map::dungeon::freeze_level_entities(&mut self.ecs);

        // Build a new map and place the player
        let current_depth = self.ecs.fetch::<Map>().depth;
        self.generate_world_map(current_depth + offset, offset);

        // Notify the player
        let mut gamelog = self.ecs.fetch_mut::<gamelog::GameLog>();
        gamelog.entries.push("You change level.".to_string());
    }
    
    fn game_over_cleanup(&mut self) {
        // Delete everything
        let mut to_delete = Vec::new();
        for e in self.ecs.entities().join() {
            to_delete.push(e);
        }
        for del in to_delete.iter() {
            self.ecs.delete_entity(*del).expect("Deletion failed @main.rs");
        }
        
        //Delete previous save
        saveload_system::delete_save();

        // Spawn a new player
        {
            let player_entity = spawner::player(&mut self.ecs, 0, 0);
            let mut player_entity_writer = self.ecs.write_resource::<Entity>();
            *player_entity_writer = player_entity;
        }

        // Replace the world maps
        self.ecs.insert(MasterDungeonMap::new());

        // Build a new map and place the player
        self.generate_world_map(1, 0);
    }

    fn generate_world_map(&mut self, new_depth : i32, offset: i32) {
        self.mapgen_index = 0;
        self.mapgen_timer = 0.0;
        self.mapgen_history.clear();
        
        let map_building_info = level_transition(&mut self.ecs, new_depth, offset);
        if let Some(history) = map_building_info {
            self.mapgen_history = history;
        } else {
            map::dungeon::thaw_level_entities(&mut self.ecs);
        }

        // The other function cannot be used here due to the borrow checker
        // Check if the map should be rejected
        let should_reject = {
            // Perform immutable operations first
            let ecs = &self.ecs;
            let has_stairs = down_stairs_present(ecs);
            let is_walkable = walkable(ecs, 11);

            // Drop the immutable borrow before performing mutable operations
            if !has_stairs || !is_walkable {
                rltk::console::log("!!!Rejected map!!!".to_string());
                true
            } else {
                false
            }
        };

        if should_reject {
            // Perform mutable operations (deleting entities) after the immutable borrow is dropped
            let to_delete = self.entities_to_remove_on_level_change();
            for target in to_delete {
                self.ecs.delete_entity(target).expect("Unable to delete entity @main.rs");
            }

            // Regenerate the map
            self.generate_world_map(new_depth, offset);
        }
    }

    fn reject_map(&mut self, ecs: &World) -> bool {

        //Delete entities from previous map
        
        let to_delete = self.entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs.delete_entity(target).expect("Unable to delete entity @main.rs");
        }

        //Check if there are stairs
        if !down_stairs_present(ecs) {
            eprintln!("Rejected: no stairs present");
            return true
        }

        let min = 15;

        //check if at least min% of tiles are walkable
        if !walkable(ecs,min){
            eprintln!("Rejected: under {}% walkable tiles", min);
            return true
        }

        false
    }
}

fn main() -> rltk::BError {
    //Installer
    initialize().expect("Failed to initialize @main.rs");
    
    // Initialize with default values
    let mut config_data = (false, false, false, true, true, true, -1f32, true, false);
    let mut load_mods:bool = false;

    // Check if the config file exists
    if config_exists() {
        // Attempt to read the config file
        match read_config() {
            Ok(data) => {
                // Update config_data with the parsed values
                config_data = data;
            }
            Err(e) => {
                // Handle the error (e.g., log it or use defaults)
                eprintln!("Failed to read config: {} @main.rs", e);
                println!("Using default config values");
            }
        }
    } else {
        eprintln!("Config file does not exist. Using default values @main.rs");
    }

    load_mods = config_data.0;
    
    // Use config_data as needed
    println!("Config values: {:?}", config_data);
    
    //TODO use all values in the config
    
    use rltk::RltkBuilder;
    let mut context = RltkBuilder::simple(80, 60)
        .unwrap()
        .with_title("Learn RLTK")
        // get from config
        .with_fullscreen(config_data.7)
        // get from config
        .with_vsync(config_data.4)
        // ignore if VSYNC is enabled. If value returned is -1, set cap to 300fps
            .with_fps_cap(if config_data.6 != -1.0 && !config_data.4 { config_data.6 } else{300f32})
        .build()?;
    
    if config_data.3 {
        context.with_post_scanlines(true);
    }
    
    if config_data.1 {
        *SHOW_MAPGEN_VISUALIZER.lock().unwrap() = true
    }
    
    if config_data.8 {
        *SHOW_MAP_BORDER.lock().unwrap() = true;
    }

    let mut gs = State {
        ecs: World::new(),
        mapgen_next_state : Some(RunState::MainMenu{ menu_selection: gui::MainMenuSelection::NewGame }),
        mapgen_index : 0,
        mapgen_history: Vec::new(),
        mapgen_timer: 0.0
    };

    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<ProvidesHealing>();
    gs.ecs.register::<InflictsDamage>();
    gs.ecs.register::<AreaOfEffect>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToUseItem>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.register::<Confusion>();
    gs.ecs.register::<SimpleMarker<SerializeMe>>();
    gs.ecs.register::<SerializationHelper>();
    gs.ecs.register::<Equippable>();
    gs.ecs.register::<Equipped>();
    gs.ecs.register::<WantsToRemoveItem>();
    gs.ecs.register::<ParticleLifetime>();
    gs.ecs.register::<HungerClock>();
    gs.ecs.register::<ProvidesFood>();
    gs.ecs.register::<MagicMapper>();
    gs.ecs.register::<Hidden>();
    gs.ecs.register::<EntryTrigger>();
    gs.ecs.register::<EntityMoved>();
    gs.ecs.register::<SingleActivation>();
    gs.ecs.register::<Door>();
    gs.ecs.register::<BlocksVisibility>();
    gs.ecs.register::<Bystander>();
    gs.ecs.register::<Vendor>();
    gs.ecs.register::<Quips>();
    gs.ecs.register::<Attributes>();
    gs.ecs.register::<Skills>();
    gs.ecs.register::<Pools>();
    gs.ecs.register::<MeleeWeapon>();
    gs.ecs.register::<Wearable>();
    gs.ecs.register::<NaturalAttackDefense>();
    gs.ecs.register::<LootTable>();
    gs.ecs.register::<Carnivore>();
    gs.ecs.register::<Herbivore>();
    gs.ecs.register::<OtherLevelPosition>();
    gs.ecs.register::<DMSerializationHelper>();
    gs.ecs.register::<LightSource>();
    gs.ecs.register::<Initiative>();
    gs.ecs.register::<MyTurn>();
    gs.ecs.register::<Faction>();
    gs.ecs.register::<SerializationHelper>();

    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());
    
    // Load NON-MODDED raws before adding to World
    raws::load_raws("/../../raws/spawns.json".parse().unwrap());

    // MUST BE THE SAME DIMENSIONS AS generate_map()
    gs.ecs.insert(MasterDungeonMap::new());
    gs.ecs.insert(Map::new(1, 56, 56, "DEFAULT NAME"));
    gs.ecs.insert(Point::new(0, 0));
    gs.ecs.insert(rltk::RandomNumberGenerator::new());
    let player_entity = spawner::player(&mut gs.ecs, 0, 0);
    gs.ecs.insert(player_entity);
    gs.ecs.insert(RunState::MapGeneration{});
    gs.ecs.insert(gamelog::GameLog{ entries : vec!["You feel drowsy...".to_string()]});
    gs.ecs.insert(particle_system::ParticleBuilder::new());
    gs.ecs.insert(rex_assets::RexAssets::new());

    gs.generate_world_map(1,0);

    rltk::main_loop(context, gs)
}