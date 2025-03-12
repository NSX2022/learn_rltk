mod directories;

//Struct for loading the simple .json mods, holds basic data
//TODO figure out how to load luau mods
//TODO Load mods alongside loading 
pub struct RawMod {
    //name of the mod
    pub name: String,
    //std::str::from_utf8 for loading the .json file
    pub data: String,
    //optional priority to set load order, lower numbers prioritized, 0 is highest priority
    pub priority: Option<i32>
}