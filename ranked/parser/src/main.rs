use chrono::prelude::*;
use regex::Regex;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};

#[derive(PartialEq)]
enum Factions {
    Sol,
    Centauri,
    Alien,
    Wildlife,
}

#[derive(Default)]
enum Modes {
    #[default]
    SolVsAlien,
    CentauriVsSol,
    CentauriVsSolVsAlien,
}

struct Player {
    player_id: i32,
    player_name: String,
    faction_type: Factions,
    is_commander: bool,
    unit_kill: Vec<i32>,
    total_unit_kills: i32,
    structure_kill: Vec<i32>,
    total_structure_kills: i32,
    death: i32,
    points: i32,
    winner: bool,
}

struct Game {
    start_time: DateTime<FixedOffset>,
    end_time: DateTime<FixedOffset>,
    current_match: Vec<String>,
    match_type: Modes,
}

//Modes consts
const SOL_VS_ALIEN: &str = "HUMANS_VS_ALIENS";
const CENTAURI_VS_SOL: &str = "HUMANS_VS_HUMANS";
const CENTAURI_VS_SOL_VS_ALIEN: &str = "HUMANS_VS_HUMANS_VS_ALIENS";

//Chat message consts
const STRUCTURE_KILL: &str = "\"structure_kill\"";
const KILLED: &str = "killed";
const JOINED_TEAM: &str = "joined team";
const CHAT: &str = "say";
const TEAM_CHAT: &str = "say_team";
const ROUND_START: &str = "World triggered \"Round_Start\"";
const ROUND_END: &str = "World triggered \"Round_Win\"";
const END_TIME: &str = "";
const START_TIME: &str = "";
const LOADING_MAP: &str = "Loading map";

//Point allocation consts
const TIER_ONE_STRUCTURE_POINTS: i32 = 10;
const TIER_TWO_STRUCTURE_POINTS: i32 = 50;
const TIER_THREE_STRUCTURE_POINTS: i32 = 100;

const TIER_ONE_UNIT_POINTS: i32 = 1;
const TIER_TWO_UNIT_POINTS: i32 = 10;
const TIER_THREE_UNIT_POINTS: i32 = 50;
const QUEEN_UNIT_POINTS: i32 = 100;

//Log range consts
const DATETIME_RANGE: std::ops::Range<usize> = 1..23;
const ROUND_START_RANGE: std::ops::Range<usize> = 25..54;
const ROUND_END_RANGE: std::ops::Range<usize> = 25..52;
const DATETIME_END: usize = 25;

//const MAP_ID = {"NarakaCity": 1, "MonumentValley": 2, "RiftBasin": 3, "Badlands": 4, "GreatErg": 5}
enum Maps {
    NarakaCity,
    MonumentValley,
    RiftBasin,
    Badlands,
    GreatErg,
}

const TIER_ONE_UNITS: &[&str] = &[
    "Crab",
    "Crab_Horned",
    "Shocker",
    "Shrimp",
    "Soldier_Rifleman",
    "Soldier_Scout",
    "Soldier_Heavy",
    "Soldier_Marksman",
    "LightQuad",
    "Wasp",
    "HoverBike",
    "Worm",
    "FlakTruck",
];
const TIER_TWO_UNITS: &[&str] = &[
    "Behemoth",
    "Hunter",
    "LightArmoredCar",
    "ArmedTransport",
    "HeavyArmoredCar",
    "TroopTransport",
    "HeavyQuad",
    "RocketLauncher",
    "PulseTank",
    "AirGunship",
    "AirFighter",
    "AirDropship",
    "Dragonfly",
    "Firebug",
    "Soldier_Commando",
    "GreatWorm",
];

const TIER_THREE_UNITS: &[&str] = &[
    "Queen",
    "Scorpion",
    "Goliath",
    "BomberCraft",
    "HeavyHarvester",
    "HoverTank",
    "RailgunTank",
    "SiegeTank",
    "AirBomber",
    "Defiler",
    "Colossus",
];

const TIER_ONE_STRUCTURES: &[&str] = &[
    "HiveSpire",
    "LesserSpawningCyst",
    "Node",
    "ThornSpire",
    "Outpost",
    "RadarStation",
    "Silo",
];

const TIER_TWO_STRUCTURES: &[&str] = &[
    "BioCache",
    "Barracks",
    "HeavyVehicleFactory",
    "LightVehicleFactory",
    "QuantumCortex",
    "GreaterSpawningCyst",
    "Refinery",
    "Bunker",
    "ConstructionSite_TurretHeavy",
    "TurretHeavy",
    "TurretMedium",
    "GrandSpawningCyst",
    "TurretAARocket",
];

const TIER_THREE_STRUCTURES: &[&str] = &[
    "ResearchFacility",
    "Nest",
    "UltraHeavyVehicleFactory",
    "Headquarters",
    "GrandSpawningCyst",
    "ColossalSpawningCyst",
    "AirFactory",
];

//impl Default for Modes{
//fn default() -> Self {
//Modes::SolVsAlien
//}
//}

impl Player {
    fn update_structure_kill(&mut self, structure: &str) {
        self.total_structure_kills += 1;
        match structure {
            s if TIER_ONE_STRUCTURES.contains(&s) => {
                self.structure_kill[0] += 1;
                self.points += TIER_ONE_STRUCTURE_POINTS;
            }
            s if TIER_TWO_STRUCTURES.contains(&s) => {
                self.structure_kill[1] += 1;
                self.points += TIER_TWO_STRUCTURE_POINTS;
            }
            s if TIER_THREE_STRUCTURES.contains(&s) => {
                self.structure_kill[2] += 1;
                self.points += TIER_THREE_STRUCTURE_POINTS;
            }
            _ => (),
        }
    }

    fn update_unit_kill(&mut self, unit: &str) {
        self.total_unit_kills += 1;
        match unit {
            u if TIER_ONE_UNITS.contains(&u) => {
                self.unit_kill[0] += 1;
                self.points += TIER_ONE_UNIT_POINTS;
            }
            u if TIER_TWO_UNITS.contains(&u) => {
                self.unit_kill[1] += 1;
                self.points += TIER_TWO_UNIT_POINTS;
            }
            u if TIER_THREE_UNITS.contains(&u) => {
                self.unit_kill[2] += 1;
                if u == "Queen" {
                    self.points += QUEEN_UNIT_POINTS;
                } else {
                    self.points += TIER_THREE_UNIT_POINTS;
                }
            }
            _ => (),
        }
    }
    fn update_death(&mut self, unit: &str) {
        self.death += 1;
        match unit {
            u if TIER_ONE_UNITS.contains(&u) => {
                self.points -= TIER_ONE_UNIT_POINTS;
            }
            u if TIER_TWO_UNITS.contains(&u) => {
                self.points -= TIER_TWO_UNIT_POINTS;
            }
            u if TIER_THREE_UNITS.contains(&u) => {
                if u == "Queen" {
                    self.points -= QUEEN_UNIT_POINTS;
                } else {
                    self.points -= TIER_THREE_UNIT_POINTS;
                }
            }
            _ => (),
        }
    }

    fn set_commander(&mut self) {
        self.is_commander = true;
    }
    fn is_fps(&self) -> bool {
        let all_sum = self.total_unit_kills + self.total_structure_kills + self.death;
        all_sum != 0
    }
    fn did_win(&mut self, winning_team: Factions) {
        self.winner = winning_team == self.faction_type;
    }
    fn __str__(&mut self) -> String {
        let player_name = &self.player_name;
        let player_id = self.player_id;
        let faction_type = match self.faction_type {
            Factions::Sol => "Sol",
            Factions::Alien => "Alien",
            Factions::Centauri => "Centauri",
            Factions::Wildlife => "Wildlife",
        };
        let unit_kills = &self.unit_kill;
        let structure_kill = &self.structure_kill;
        let deaths = self.death;
        let winner = self.winner;
        let is_infantry = self.is_fps();
        let is_commander = self.is_commander;
        let points = self.points;
        format!("name: {player_name}, id: {player_id}, faction_type: {faction_type}, unit_kills: {unit_kills:?},
                structure_kill: {structure_kill:?}, deaths = {deaths} self.winner = {winner} is_infantry = {is_infantry}
                is_commander = {is_commander} points= {points}")
    }
}

fn is_valid_faction_type(match_type: Modes, faction_type: Factions) -> bool {
    match match_type {
        Modes::SolVsAlien => faction_type != Factions::Centauri,
        Modes::CentauriVsSol => faction_type != Factions::Alien,
        _ => true,
    }
}

impl Game {
    fn get_current_match(&mut self, all_lines: &[String]) {
        let mut did_find_world_win = false;
        //TODO improve this part
        let mut end_index = 0;
        for (i, value) in all_lines.iter().rev().enumerate() {
            if value[ROUND_END_RANGE].trim() == ROUND_END {
                self.end_time = match DateTime::parse_from_str(
                    value[DATETIME_RANGE].trim(),
                    "%m/%d/%Y - %H:%M:%S",
                ) {
                    Ok(datetime) => datetime,
                    Err(e) => panic!("Error in trying to parse round start time {e}"),
                };
                did_find_world_win = true;
                end_index = all_lines.len() - i;
                //START HERE
            } else if value[ROUND_START_RANGE].trim() == ROUND_START && did_find_world_win {
                self.start_time = match DateTime::parse_from_str(
                    value[DATETIME_RANGE].trim(),
                    "%m/%d/%Y - %H:%M:%S",
                ) {
                    Ok(datetime) => datetime,
                    Err(e) => panic!("Error in trying to parse round start time {e}"),
                };
                self.current_match = all_lines[all_lines.len() - i - 1..end_index].to_vec();
                println!("{:#?}", self.current_match);
            }
        }
    }

    fn get_match_type(&mut self) {
        //TODO const this
        let match_type_thing = self.current_match[0][54..].trim();
        //TODO check this
        let match_type_regex = Regex::new(r#"\(gametype "(.*?)"\)"#).unwrap();
        let match_type = match_type_regex.find(match_type_thing).unwrap().as_str();
        if match_type == SOL_VS_ALIEN {
            self.match_type = Modes::SolVsAlien
        } else if match_type == CENTAURI_VS_SOL {
            self.match_type = Modes::CentauriVsSol
        } else if match_type == CENTAURI_VS_SOL_VS_ALIEN {
            self.match_type = Modes::CentauriVsSolVsAlien
        }
    }

    fn get_current_map(&mut self, all_lines: &[String]) {
        let mut req_info = all_lines
            .iter()
            .filter(|x| remove_chat_messages(x))
            .map(|x| remove_date_data(x))
            .filter(|x| x.contains(LOADING_MAP)).rev();

        let required_line = match req_info.next() {
            Some(map_line) => map_line,
            None => panic!("Couldn't find the current map"),
        };


        let map_regex = Regex::new(r#"Loading map "(.*)""#).unwrap();
        let map = match map_regex.captures(required_line).unwrap().get(1) {
            Some(map) => map.as_str(),
            None => panic!("Couldn't find the current map/regex wrong maybe")
        };
        //let map = map_regex.captures_iter(required_line).).map(|x| x.extract());

        println!("the map is {}", map);
    }
}
impl Default for Game {
    fn default() -> Self {
        //Make this better, ugly for now
        let utc_now = Utc::now();
        let fixed_offset = FixedOffset::east_opt(0).unwrap(); // UTC offset as example
        let default_time = utc_now.with_timezone(&fixed_offset);
        Game {
            start_time: default_time,
            end_time: default_time,
            current_match: Vec::new(),
            match_type: Modes::default(),
        }
    }
}

fn remove_chat_messages(line: &str) -> bool {
    let mut words = line.split_whitespace();
    let chat_keywords = [CHAT, TEAM_CHAT];
    !words.any(|i| chat_keywords.contains(&i))
}

fn remove_date_data(line: &str) -> &str {
    if line.len() > DATETIME_END {
        &line.trim()[DATETIME_END..]
    } else {
        ""
    }
}

//fn get_structure_killed_filter(all_req) -> :
//return filter(lambda x: STRUCTURE_KILL in x.split(" "), all_req)
fn get_structure_kills(lines: Vec<&str>) -> Vec<&str> {
    lines
        .into_iter()
        .filter(|line| line.split_whitespace().any(|word| word == STRUCTURE_KILL))
        .collect()
}

fn get_kills(lines: Vec<&str>) -> Vec<&str> {
    lines
        .into_iter()
        .filter(|line| line.split_whitespace().any(|word| word == KILLED))
        .collect()
}

fn checking_folder(path: String) {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => panic!("Failed to read directory"),
    };
    let file_entries = entries
        .map(|r| r.unwrap())
        .filter(|r| r.path().is_file())
        .map(|r| r.path());
    let mut log_files: Vec<_> = file_entries
        .filter(|r| r.extension().unwrap_or(OsStr::new("")) == "log")
        .collect();
    log_files.sort();
    let mut all_lines: Vec<_> = Vec::new();
    for file in log_files {
        let reader = match File::open(file) {
            Ok(open_file) => BufReader::new(open_file),
            Err(_) => panic!("Error in opening the log file"),
        };
        for line in reader.lines() {
            match line {
                Ok(result) => all_lines.push(result),
                Err(_) => println!("Could not read a line"),
            }
        }
    }
    let mut game = Game::default();
    game.get_current_map(&all_lines);
    game.get_current_match(&all_lines);
    game.get_match_type();
}

fn main() {
    checking_folder("/home/neeladri/Silica/ranked/log_folder/".to_string());
}
