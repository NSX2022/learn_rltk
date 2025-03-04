use std::fs;
use std::fs::File;
use std::io::Write; // Import the Write trait
use std::path::{PathBuf};
use std::env;

// Constants for directory and file names
const MODS_DIR: &str = "mods";
const CONFIG_DIR: &str = "config";
const RAWS_DIR: &str = "raws";
const INFO_DIR: &str = "info";
const MODDED_ENTITIES_FILE: &str = "modded_entities.json";
const README_FILE: &str = "README.txt";
const CONFIG_FILE: &str = "config.json";

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
    let info_dir = mods_dir.join(INFO_DIR);
    let modded_entities_path = raws_dir.join(MODDED_ENTITIES_FILE);
    let readme_path = info_dir.join(README_FILE);
    let config_file_path = config_dir.join(CONFIG_FILE);

    // Create directories
    fs::create_dir_all(&raws_dir)?;
    fs::create_dir_all(&info_dir)?;
    fs::create_dir_all(&config_dir)?;

    // Create files if they don't exist
    if !modded_entities_path.exists() {
        File::create(&modded_entities_path)?;
    }

    if !readme_path.exists() {
        File::create(&readme_path)?;
    }

    if !config_file_path.exists() {
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
        writeln!(file, "Syntax guide:")?;
        writeln!(file, "TODO")?;
        
        println!("README.txt created");
    } else {
        println!("README.txt already exists");
    }

    // Write to config.json only if it's empty
    let metadata = fs::metadata(&config_file_path)?;
    if metadata.len() == 0 {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&config_file_path)?;
        writeln!(file, "LOAD_MODS = false")?;
        //TODO make Main use this
        writeln!(file, "SHOW_MAP_VISUALIZER = false")?;

        println!("config.json created");
    } else {
        println!("config.json already exists")
    }

    println!("Directories and files created successfully at: {:?}", exe_dir);
    Ok(())
}

fn main() {
    if let Err(e) = initialize() {
        eprintln!("Error during initialization: {}", e);
    }
}