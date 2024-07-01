use chrono::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};

#[derive(PartialEq, Default, Hash, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum Factions {
    #[default]
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

#[derive(Debug)]
struct Player {
    player_id: i64,
    player_name: String,
    faction_type: Factions,
    is_commander: bool,
    unit_kill: [i32; 3],
    total_unit_kills: i32,
    structure_kill: [i32; 3],
    total_structure_kills: i32,
    death: i32,
    points: i32,
    winner: bool,
}

struct Game {
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    current_match: Vec<String>,
    match_type: Modes,
    map: Maps,
    winning_team: Factions,
    players: HashMap<(i64, Factions), Player>,
}

//Modes consts
//const MAP_HASH: HashMap<&str, Maps> = HashMap::from([
//("NarakaCity", Maps::NarakaCity),
//("MonumentValley", Maps::MonumentValley),
//("Badlands", Maps::Badlands),
//("GreatErg", Maps::GreatErg),
//("RiftBasin", Maps::RiftBasin)
//]);

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
const TRIGGERED: &str = "triggered";

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
#[derive(Debug, Default)]
enum Maps {
    #[default]
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
    fn new(player_id: i64, player_name: String, faction_type: Factions) -> Self {
        Player {
            player_id,
            player_name,
            faction_type,
            is_commander: false,
            unit_kill: [0, 0, 0],
            total_unit_kills: 0,
            structure_kill: [0, 0, 0],
            total_structure_kills: 0,
            death: 0,
            points: 0,
            winner: false,
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

fn get_byte_indices(line: String, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let valid_start = line
        .char_indices()
        .nth(range.start)
        .map(|(i, _)| i)
        .unwrap_or(0);

    let valid_end = line
        .char_indices()
        .nth(range.end)
        .map(|(i, _)| i)
        .unwrap_or(line.len());

    valid_start..valid_end
}

impl Game {
    fn get_factions(faction_name: &str) -> Factions {
        let faction_type: Factions;
        if faction_name == "Sol" {
            faction_type = Factions::Sol;
        } else if faction_name == "Alien" {
            faction_type = Factions::Alien;
        } else if faction_name == "Centauri" {
            faction_type = Factions::Centauri;
        } else {
            faction_type = Factions::Wildlife;
        }
        faction_type
    }
    fn get_all_players(&mut self) {
        let joined_team_lines = self
            .current_match
            .iter()
            .filter(|x| remove_chat_messages(x))
            .map(|x| remove_date_data(x))
            .filter(|x| x.contains(JOINED_TEAM));

        let join_match_regex =
            Regex::new(r#""(.*?)<(.*?)><(.*?)><(.*?)>" joined team "(.*)""#).unwrap();

        for line in joined_team_lines {
            let joined_player = join_match_regex.captures(line);
            let Some((_, [player_name, _, player_id, _, player_faction])) =
                joined_player.map(|caps| caps.extract())
            else {
                continue;
            };
            let faction_type: Factions;
            if player_faction == "Sol" {
                faction_type = Factions::Sol;
            } else if player_faction == "Alien" {
                faction_type = Factions::Alien;
            } else if player_faction == "Centauri" {
                faction_type = Factions::Centauri;
            } else {
                continue;
            }

            self.players.insert(
                (player_id.parse::<i64>().unwrap(), faction_type),
                Player::new(
                    player_id.parse::<i64>().unwrap(),
                    player_name.to_string(),
                    faction_type,
                ),
            );
        }
    }
    fn get_current_match(&mut self, all_lines: &[String]) {
        let mut did_find_world_win = false;
        //TODO improve this part
        let mut end_index = 0;
        for (i, value) in all_lines.iter().rev().enumerate() {
            let byte_matched_round_end_range = get_byte_indices(value.to_string(), ROUND_END_RANGE);
            let byte_matched_round_start_range =
                get_byte_indices(value.to_string(), ROUND_START_RANGE);
            let byte_matched_datetime_range = get_byte_indices(value.to_string(), DATETIME_RANGE);
            if value[byte_matched_round_end_range].trim() == ROUND_END {
                self.end_time = match NaiveDateTime::parse_from_str(
                    value[byte_matched_datetime_range].trim(),
                    "%m/%d/%Y - %H:%M:%S",
                ) {
                    Ok(datetime) => datetime,
                    Err(e) => {
                        panic!("Error in trying to parse round start time: {e}")
                    }
                };
                did_find_world_win = true;
                end_index = all_lines.len() - i;
            } else if value[byte_matched_round_start_range].trim() == ROUND_START
                && did_find_world_win
            {
                self.start_time = match NaiveDateTime::parse_from_str(
                    value[DATETIME_RANGE].trim(),
                    "%m/%d/%Y - %H:%M:%S",
                ) {
                    Ok(datetime) => datetime,
                    Err(e) => panic!("Error in trying to parse round start time {e}"),
                };
                self.current_match = all_lines[all_lines.len() - i - 1..end_index].to_vec();
                return;
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

    fn process_kills(&mut self) {
        //TODO make it optimized by using normal for loop or something else.
        let kill_lines = self
            .current_match
            .clone()
            .into_iter()
            .filter(|line| line.contains(KILLED));

        let kill_regex = match Regex::new(
            r#""(.*?)<(.*?)><(.*?)><(.*?)>" killed "(.*?)<(.*?)><(.*?)><(.*?)>" with "(.*)" \(dmgtype "(.*)"\) \(victim "(.*)"\)"#,
        ) {
            Ok(kill_regex) => kill_regex,
            Err(e) => panic!("Error in creating the kill regex: {e}"),
        };

        for kill_line in kill_lines {
            let kill_matches = kill_regex.captures(&kill_line);
            let Some((
                _,
                [player_name, _, player_id, player_faction, enemy_name, _, enemy_id, enemy_faction, _, _, victim],
            )) = kill_matches.map(|cap| cap.extract())
            else {
                continue;
            };

            let faction_type = Game::get_factions(player_faction);

            match player_id.parse::<i64>() {
                Ok(player_id) => {
                    let player = self
                        .players
                        .entry((player_id, faction_type))
                        .or_insert_with(|| {
                            Player::new(player_id, player_name.to_string(), faction_type)
                        });
                    player.update_unit_kill(victim);
                }
                Err(_) => {
                    //change this, unnecessary thing
                    //println!("Can't parse due to {e}");
                }
            };


            let enemy_faction_type = Game::get_factions(enemy_faction);
            match enemy_id.parse::<i64>() {
                Ok(enemy_id) => {
                    let enemy_player = self
                        .players
                        .entry((enemy_id, enemy_faction_type))
                        .or_insert_with(|| {
                            Player::new(enemy_id, enemy_name.to_string(), enemy_faction_type)
                        });
                    enemy_player.update_death(victim);
                }
                Err(_) => {
                    //println!("Can't parse due to {e}");
                }
            };
        }
    }

    fn process_structure_kills(&mut self) {
        //TODO make it optimized by using normal for loop or something else.
        let kill_lines = self
            .current_match
            .clone()
            .into_iter()
            .filter(|line| line.contains(STRUCTURE_KILL));

        let kill_regex = match Regex::new(
            r#""(.*?)<(.*?)><(.*?)><(.*?)>" triggered "structure_kill" \(structure "(.*)"\) \(struct_team "(.*)"\)"#,
        ) {
            Ok(kill_regex) => kill_regex,
            Err(e) => panic!("Error in creating the kill regex: {e}"),
        };

        for kill_line in kill_lines {
            let kill_matches = kill_regex.captures(&kill_line);
            let Some((
                _,
                [player_name, _, player_id, player_faction, enemy_structure, _],
            )) = kill_matches.map(|cap| cap.extract())
            else {
                continue;
            };

            let faction_type = Game::get_factions(player_faction);

            match player_id.parse::<i64>() {
                Ok(player_id) => {
                    let player = self
                        .players
                        .entry((player_id, faction_type))
                        .or_insert_with(|| {
                            Player::new(player_id, player_name.to_string(), faction_type)
                        });
                    player.update_structure_kill(enemy_structure);
                }
                Err(_) => {
                    //change this, unnecessary thing
                    //println!("Can't parse due to {e}");
                }
            };

        }
    }

    fn get_current_map(&mut self, all_lines: &[String]) {
        let req_info = all_lines
            .iter()
            .filter(|x| remove_chat_messages(x))
            .map(|x| remove_date_data(x))
            .filter(|x| x.contains(LOADING_MAP))
            .rev();

        let map_regex = match Regex::new(r#"Loading map "(.*)""#) {
            Ok(map_regex) => map_regex,
            Err(_) => panic!("Error in creating the get_current_map_regex"),
        };

        for required_line in req_info {
            let map_matched = map_regex.captures(required_line);
            match map_matched {
                Some(map) => {
                    let map_str = map.get(1).unwrap().as_str();
                    if map_str == "NarakaCity" {
                        self.map = Maps::NarakaCity
                    } else if map_str == "MonumentValley" {
                        self.map = Maps::MonumentValley
                    } else if map_str == "RiftBasin" {
                        self.map = Maps::RiftBasin
                    } else if map_str == "Badlands" {
                        self.map = Maps::Badlands
                    } else if map_str == "GreatErg" {
                        self.map = Maps::GreatErg
                    }
                    return;
                }
                None => continue,
            }
        }
    }

    fn get_winning_team(&mut self) {
        let winning_team_log = self
            .current_match
            .iter()
            .rev()
            .filter(|x| x.contains(TRIGGERED));

        let victory_regex = match Regex::new(r#"Team "(.*?)" triggered "Victory""#) {
            Ok(map_regex) => map_regex,
            Err(e) => panic!("Error in creating the get_current_map_regex due to: {e}"),
        };

        for line in winning_team_log {
            let victory_matched = victory_regex.captures(line);
            match victory_matched {
                Some(winning_match) => {
                    let winning_team_str = winning_match.get(1).unwrap().as_str();
                    if winning_team_str == "Alien" {
                        self.winning_team = Factions::Alien;
                    } else if winning_team_str == "Wildlife" {
                        self.winning_team = Factions::Wildlife;
                    } else if winning_team_str == "Sol" {
                        self.winning_team = Factions::Sol;
                    } else if winning_team_str == "Centauri" {
                        self.winning_team = Factions::Centauri;
                    }
                    return;
                }
                None => continue,
            }
        }
    }
}
impl Default for Game {
    fn default() -> Self {
        //Make this better, ugly for now
        let default_time = NaiveDateTime::default();
        Game {
            start_time: default_time,
            end_time: default_time,
            current_match: Vec::new(),
            match_type: Modes::default(),
            map: Maps::default(),
            winning_team: Factions::default(),
            players: HashMap::new(),
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
        let byte_corrected_datetimeend =
            get_byte_indices(line.to_string(), DATETIME_END..line.len());
        &line.trim()[byte_corrected_datetimeend]
    } else {
        ""
    }
}

//fn get_structure_killed_filter(all_req) -> :
//return filter(lambda x: STRUCTURE_KILL in x.split(" "), all_req)
fn get_structure_kills(lines: Vec<&str>) -> Vec<&str> {
    lines
        .into_iter()
        .filter(|line| line.contains(STRUCTURE_KILL))
        .collect()
}

fn get_kills(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| line.contains(KILLED))
        .collect()
}

fn parse_info(all_lines: Vec<String>) {
    let mut game = Game::default();
    game.get_current_map(&all_lines);
    game.get_current_match(&all_lines);
    game.get_match_type();
    game.get_winning_team();
    game.get_all_players();
    game.process_kills();
    game.process_structure_kills();
    println!("{:?}", game.players);
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
            Err(e) => panic!("Error in opening the log file due to: {e}"),
        };
        for line in reader.lines() {
            match line {
                Ok(result) => all_lines.push(result),
                Err(e) => println!("Could not read a line due to: {e}"),
            }
        }
    }
    parse_info(all_lines);
}

fn main() {
    checking_folder("/home/neeladri/Silica/ranked/log_folder/".to_string());
}
