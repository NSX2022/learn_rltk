mod directories;

//Struct for loading the simple .json mods
//TODO figure out how to load luau mods
//TODO Load mods alongside loading 
pub struct RawMod {
    //name of the mod
    pub name: String,
    //std::str::from_utf8 for loading the .json file
    pub data: String,
    //optional priority to set load order, lower numbers priority, 0 is highest priority
    priority: Option<i32>
}