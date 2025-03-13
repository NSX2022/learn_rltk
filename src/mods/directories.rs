use std::{fs, io};
use std::any::Any;
use std::fs::File;
use std::io::{BufRead, Write};
use std::path::{PathBuf};
use std::env;
use std::ptr::write;
use lazy_static::lazy_static;

// Constants for directory and file names
const MODS_DIR: &str = "mods";
const CONFIG_DIR: &str = "config";
const RAWS_DIR: &str = "raws";
const INFO_DIR: &str = "info";
const LUA_DIR: &str = "lua";
const MODDED_ENTITIES_FILE: &str = "example_mod.json";
const README_FILE: &str = "README.txt";
const CONFIG_FILE: &str = "config.txt";

pub fn initialize() -> Result<(), std::io::Error> {
    println!("Initializing directory creation");

    // Get the directory of the current executable
    let exe_dir = env::current_exe()?
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Could not get executable directory"))?
        .to_path_buf();

    // Construct paths relative to the executable directory
    let config_dir = exe_dir.join(CONFIG_DIR);
    let mods_dir = exe_dir.join(MODS_DIR);
    let raws_dir = mods_dir.join(RAWS_DIR);
    let lua_dir = mods_dir.join(LUA_DIR);
    let example_mod_path = raws_dir.join(MODDED_ENTITIES_FILE);
    let readme_path = exe_dir.join(README_FILE);
    let config_file_path = config_dir.join(CONFIG_FILE);

    // Create directories
    fs::create_dir_all(&raws_dir)?;
    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(&lua_dir)?;

    // Create files if they don't exist
    if !example_mod_path.exists() {
        File::create(&example_mod_path)?;
    }

    if !readme_path.exists() {
        File::create(&readme_path)?;
    }

    if !config_exists() {
        File::create(&config_file_path)?;
    }

    // Write to README.txt only if it's empty
    let metadata = fs::metadata(&readme_path)?;
    if metadata.len() == 0 {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&readme_path)?;
        writeln!(file, "This is the SIMPLIFIED modding platform allowing you to load custom .json files")?;
        writeln!(file, "You need to use correct syntax or the game will crash while loading your mod")?;
        writeln!(file, "For more advanced modding (e.g, writing a new UI or changing game systems), you need to use luau files")?;
        writeln!(file, "To use ANY mods, lua or otherwise, you must change LOAD_MODS to true in the config")?;
        writeln!(file)?;
        writeln!(file, "!WARNING! example_mod.json will be restored every launch if you delete it. To disable this mod, delete all of its contents so it's not loaded")?;
        writeln!(file)?;
        writeln!(file, "Syntax guide:")?;
        writeln!(file, "TODO")?;
        
        println!("README.txt created");
    } else {
        println!("README.txt already exists");
    }

    // Write to config.txt only if it's empty
    let metadata = fs::metadata(&config_file_path)?;
    /* check number of lines to ensure that config wasn't modified? 
        eprinln!() an error in case lua mods add stuff to the config?
     */
    if metadata.len() == 0 {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&config_file_path)?;
        /*TODO make Main use this via calling read_config and using that function's Tuple return to use as settings 
            DO THIS AFTER CALLING initialize() to ensure that config.txt exists
            Skip lines in the config file that start with //
         */
        writeln!(file, "//all values with <RANGE = 0-1> are true or false, 1 = true and 0 = false")?;
        writeln!(file, "//whether or not to load lua and json mods [DEFAULT = 0] <RANGE = 0-1>")?;
        writeln!(file, "0")?;
        writeln!(file, "//whether or not to show the map creation visualizer [DEFAULT = 0] <RANGE = 0-1>")?;
        writeln!(file, "0")?;
        //Ch 57 in the documentation or something
        writeln!(file, "//whether or not to show FPS [DEFAULT = 0] <RANGE = 0-1>")?;
        writeln!(file, "0")?;

        writeln!(file, "//whether or not to use scanlines shader [DEFAULT = 1] <RANGE = 0-1>")?;
        writeln!(file, "1")?;
        writeln!(file, "//whether or not to use VSYNC [DEFAULT = 1] <RANGE = 0-1>")?;
        writeln!(file, "1")?;
        //Also Ch 57 I think
        writeln!(file,"//SEVERE PERFORMANCE IMPACT")?;
        writeln!(file,"//whether or not to use multithreading [DEFAULT = 1] <RANGE = 0-1>")?;
        writeln!(file,"1")?;
        
        writeln!(file,"//frame limit [DEFAULT = -1] <RANGE = -1-(2^31) -1> CANNOT BE 0")?;
        writeln!(file,"//set to -1 for no limit")?;
        writeln!(file,"-1")?;
        
        writeln!(file,"//whether or not to use fullscreen [DEFAULT = 1] <RANGE = 0-1>")?;
        writeln!(file,"1")?;

        writeln!(file,"//whether or not to show map boarder [DEFAULT = 0] <RANGE = 0-1>")?;
        writeln!(file,"0")?;
        //Add more for verbose logging and such

        println!("config.txt created");
    } else {
        println!("config.txt already exists")
    }

    println!("Directories and files initialized successfully at: {:?}", exe_dir);
    Ok(())
}

// MUST BE UPDATED every time you add a new config option
pub fn config_defaults() -> (bool,bool,bool,bool,bool,bool,f32, bool) { 
    (false,false,false,true,true,true,-1f32, true)
}

pub fn read_config() -> Result<(bool, bool, bool, bool, bool, bool, f32, bool, bool), io::Error> {
    // Get the directory of the current executable
    let exe_dir = env::current_exe()?
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Could not get executable directory @directories.rs"))?
        .to_path_buf();

    // Construct the path to the config file
    let config_file_path = exe_dir.join(CONFIG_DIR).join(CONFIG_FILE);

    // Open the config file
    let file = fs::File::open(&config_file_path)?;
    let reader = io::BufReader::new(file);

    // Initialize variables to hold the config values
    let mut load_mods = false;
    let mut show_map_creation_visualizer = false;
    let mut show_fps = false;
    let mut use_scanlines_shader = true;
    let mut use_vsync = true;
    let mut use_multithreading = true;
    let mut frame_limit = -1f32;
    let mut use_fullscreen = true;
    let mut show_border = false;

    // Read the file line by line
    for (line_number, line) in reader.lines().enumerate() {
        let read_line = line?;

        // Skip lines that start with "//" (comments)
        if read_line.trim_start().starts_with("//") || read_line.is_empty() {
            continue;
        }

        // Parse the line based on its position in the file
        eprintln!("Extracted {} from config",&read_line);
        match line_number {
            //NUMBERS NEED TO BE PRECISE WITH WHICH LINE EACH SETTING IS SET ON
            //TODO FIX so that it goes by numbers of values collected, not specific lines, to no longer necessitate hard-coding values
            2 => {
                load_mods = parse_bool(&read_line)?;
            }
            4 => {
                show_map_creation_visualizer = parse_bool(&read_line)?;
            }
            6 => {
                show_fps = parse_bool(&read_line)?;
            }
            8 => {
                use_scanlines_shader = parse_bool(&read_line)?;
            }
            10 => {
                use_vsync = parse_bool(&read_line)?;
            }
            13 => {
                use_multithreading = parse_bool(&read_line)?;
            }
            16 => {
                frame_limit = parse_f32(&read_line)?;
            }
            18 => {
                use_fullscreen = parse_bool(&read_line)?;
            }
            20 => {
                show_border = parse_bool(&read_line)?;
            }
            
            _ => {
                // Ignore extra lines
                continue;
            }
        }
    }

    Ok((
        load_mods,
        show_map_creation_visualizer,
        show_fps,
        use_scanlines_shader,
        use_vsync,
        use_multithreading,
        frame_limit,
        use_fullscreen,
        show_border
    ))
}

// helpers :)
fn parse_f32(s: &str) -> Result<f32, io::Error> {
    s.trim()
        .parse::<f32>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn parse_bool(s: &str) -> Result<bool, io::Error> {
    match s.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Expected 0 or 1, found: {}", s),
        )),
    }
}

pub fn config_exists() -> bool {
    let exe_dir = env::current_exe().expect("Failed to get path to executable directory")
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Could not get executable directory")).expect("")
        .to_path_buf();
    
    let config_dir = exe_dir.join(CONFIG_DIR);
    let config_file_path = config_dir.join(CONFIG_FILE);
    
    config_file_path.exists()
}