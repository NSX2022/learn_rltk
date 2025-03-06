use std::fs;
use std::fs::File;
use std::io::Write;
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

        writeln!(file,"//frame limit [DEFAULT = -1] <RANGE = -1-2^31>")?;
        writeln!(file,"-1")?;
        //Add more for verbose logging and such

        println!("config.txt created");
    } else {
        println!("config.txt already exists")
    }

    println!("Directories and files initialized successfully at: {:?}", exe_dir);
    Ok(())
}

// MUST BE UPDATED every time you add a new config option
pub fn config_defaults() -> (bool,bool,bool,bool,bool,bool,f32) { 
    (false,false,false,true,true,true,-1f32)
}

pub fn read_config() -> (bool,bool,bool,bool,bool,bool,f32) {
    // PLACEHOLDER, has default values
    let to_ret = config_defaults();
    //TODO actually read config.txt, set to_ret to those values
    
    
    to_ret
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