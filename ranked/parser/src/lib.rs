use chrono::{prelude::*, TimeDelta};
use log::{error, info, warn};
use regex::Regex;
use rev_lines::RevLines;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Default, Hash, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum Factions {
    #[default]
    Sol,
    Centauri,
    Alien,
    Wildlife,
}

#[derive(Default, Hash, PartialEq, Eq, Debug)]
pub enum Modes {
    #[default]
    SolVsAlien,
    CentauriVsSol,
    CentauriVsSolVsAlien,
}

#[derive(Default, Debug)]
struct CommanderDataStructure {
    current_commander: HashMap<(i64, Factions), NaiveDateTime>,
    commander_faction: HashMap<Factions, i64>,
    commander_time: HashMap<(i64, Factions), TimeDelta>,
}

impl CommanderDataStructure {
    fn add_commander_start(
        &mut self,
        player_id: i64,
        faction_type: Factions,
        time_start: NaiveDateTime,
    ) {
        self.commander_faction.insert(faction_type, player_id);
        // TODO No checks for if there was a previous commander in the position already
        self.current_commander
            .insert((player_id, faction_type), time_start);
    }

    fn add_commander_end(
        &mut self,
        player_id: i64,
        faction_type: Factions,
        time_end: NaiveDateTime,
    ) {
        if let Some(time_start) = self.current_commander.remove(&(player_id, faction_type)) {
            let duration = time_end.signed_duration_since(time_start);
            let _ = self
                .commander_time
                .entry((player_id, faction_type))
                .and_modify(|e| {
                    *e += duration;
                })
                .or_insert(duration);
        }
    }

    fn is_current_commander(&self, faction_type: Factions, player_id: i64) -> bool {
        match self.commander_faction.get(&faction_type) {
            Some(&commander) => commander == player_id,
            None => false,
        }
    }

    fn get_all_commander(&self) -> HashMap<Factions, i64> {
        let mut max_duration: HashMap<Factions, (i64, TimeDelta)> = HashMap::new();
        for ((id, faction), time_delta) in &self.commander_time {
            max_duration
                .entry(*faction)
                .and_modify(|e| {
                    if e.1 < *time_delta {
                        *e = (id.to_owned(), time_delta.to_owned())
                    }
                })
                .or_insert((*id, *time_delta));
        }
        let mut to_return: HashMap<Factions, i64> = HashMap::new();
        for (faction, (player_id, _)) in max_duration {
            to_return.insert(faction, player_id);
        }
        to_return
    }
}

#[derive(Debug)]
pub struct Player {
    pub player_id: i64,
    pub player_name: String,
    pub faction_type: Factions,
    pub is_commander: bool,
    pub unit_kill: [i32; 3],
    pub total_unit_kills: i32,
    pub structure_kill: [i32; 3],
    pub total_structure_kills: i32,
    pub death: i32,
    pub points: i32,
    pub winner: bool,
    is_in_game: bool,
    pub last_entered_time: NaiveDateTime,
    pub last_left_time: NaiveDateTime,
    pub duration_played: TimeDelta,
}

#[derive(Debug)]
pub struct Game {
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub current_match: Vec<String>,
    pub match_type: Modes,
    pub map: Maps,
    pub winning_team: Factions,
    pub players: HashMap<(i64, Factions), Player>,
}

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
const LOADING_MAP: &str = "Loading map";
const TRIGGERED: &str = "triggered";
const CHANGED_ROLE: &str = "changed role";
const DISCONNECTED: &str = "disconnected";

//Point allocation consts
const TIER_ONE_STRUCTURE_POINTS: i32 = 10;
const TIER_TWO_STRUCTURE_POINTS: i32 = 50;
const TIER_THREE_STRUCTURE_POINTS: i32 = 100;
pub const TIER_ONE: usize = 0;
pub const TIER_TWO: usize = 1;
pub const TIER_THREE: usize = 2;

const TIER_ONE_UNIT_POINTS: i32 = 1;
const TIER_TWO_UNIT_POINTS: i32 = 10;
const TIER_THREE_UNIT_POINTS: i32 = 50;
const QUEEN_UNIT_POINTS: i32 = 100;

//Log range consts
const DATETIME_RANGE: std::ops::Range<usize> = 1..23;
const ROUND_START_RANGE: std::ops::Range<usize> = 25..54;
const ROUND_END_RANGE: std::ops::Range<usize> = 25..52;
const MATCH_TYPE_RANGE: usize = 54;
const DATETIME_END: usize = 25;

#[derive(Debug, Default, Hash, Eq, PartialEq)]
pub enum Maps {
    #[default]
    NarakaCity,
    MonumentValley,
    RiftBasin,
    Badlands,
    GreatErg,
    TheMaw,
    CrimsonPeak,
    NorthPolarCap,
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
    "Squid",
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
    "HeavyTurret",
    "Turret",
    "GrandSpawningCyst",
    "AntiAirRocketTurret",
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

impl Player {
    fn update_structure_kill(&mut self, structure: &str) {
        self.total_structure_kills += 1;
        match structure {
            s if TIER_ONE_STRUCTURES.contains(&s) => {
                self.structure_kill[TIER_ONE] += 1;
                self.points += TIER_ONE_STRUCTURE_POINTS;
            }
            s if TIER_TWO_STRUCTURES.contains(&s) => {
                self.structure_kill[TIER_TWO] += 1;
                self.points += TIER_TWO_STRUCTURE_POINTS;
            }
            s if TIER_THREE_STRUCTURES.contains(&s) => {
                self.structure_kill[TIER_THREE] += 1;
                self.points += TIER_THREE_STRUCTURE_POINTS;
            }
            _ => (),
        }
    }

    fn update_unit_kill(&mut self, unit: &str, enemy_faction: Factions) {
        let is_enemy = if enemy_faction != self.faction_type {
            1
        } else {
            -1
        };
        self.total_unit_kills += is_enemy;
        match unit {
            u if TIER_ONE_UNITS.contains(&u) => {
                self.unit_kill[TIER_ONE] += is_enemy;
                self.points += TIER_ONE_UNIT_POINTS * is_enemy;
            }
            u if TIER_TWO_UNITS.contains(&u) => {
                self.unit_kill[TIER_TWO] += is_enemy;
                self.points += TIER_TWO_UNIT_POINTS * is_enemy;
            }
            u if TIER_THREE_UNITS.contains(&u) => {
                self.unit_kill[TIER_THREE] += is_enemy;
                if u == "Queen" {
                    self.points += QUEEN_UNIT_POINTS * is_enemy;
                } else {
                    self.points += TIER_THREE_UNIT_POINTS * is_enemy;
                }
            }
            _ => (),
        }
    }

    fn new(
        player_id: i64,
        player_name: String,
        faction_type: Factions,
        last_entered_time: NaiveDateTime,
        last_left_time: NaiveDateTime,
        is_in_game: bool,
    ) -> Self {
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
            last_entered_time,
            last_left_time,
            duration_played: TimeDelta::zero(),
            is_in_game,
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

    pub fn set_commander(&mut self) {
        self.is_commander = true;
    }
    pub fn is_fps(&self) -> bool {
        let all_sum = self.total_unit_kills + self.total_structure_kills + self.death;
        all_sum != 0
    }
    pub fn did_win(&self, winning_team: Factions) -> bool {
        //self.winner = winning_team == self.faction_type;
        winning_team == self.faction_type
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

fn _is_valid_faction_type(match_type: Modes, faction_type: Factions) -> bool {
    match match_type {
        Modes::SolVsAlien => faction_type != Factions::Centauri,
        Modes::CentauriVsSol => faction_type != Factions::Alien,
        _ => true,
    }
}

fn get_byte_indices(line: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
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
    pub fn process_player_durations(&mut self) {
        for player in self.players.values_mut() {
            if player.is_in_game {
                player.last_left_time = self.end_time;
                player.duration_played += player.last_left_time - player.last_entered_time;
            }
        }
    }

    pub fn get_player_vec(&self) -> Vec<&Player> {
        self.players.values().collect()
    }

    pub fn get_match_length(&self) -> i32 {
        (self.end_time - self.start_time)
            .num_seconds()
            .try_into()
            .unwrap_or_else(|e| panic!("Time couldnt be converted to i32 due to {e}"))
    }

    pub fn _get_playing_factions(&self) -> Vec<Factions> {
        match self.match_type {
            Modes::SolVsAlien => [Factions::Sol, Factions::Alien].to_vec(),
            Modes::CentauriVsSol => [Factions::Centauri, Factions::Sol].to_vec(),
            Modes::CentauriVsSolVsAlien => {
                [Factions::Sol, Factions::Alien, Factions::Centauri].to_vec()
            }
        }
    }

    pub fn get_factions(faction_name: &str) -> Factions {
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

    pub fn get_commanders(&mut self) {
        let role_change_pattern =
            Regex::new(r#""(.*?)<(.*?)><(.*?)><(.*?)>" changed role to "(.*?)""#).unwrap();

        let player_disconnect = Regex::new(r#""(.*?)<(.*?)><(.*?)><(.*?)>" disconnected"#).unwrap();

        let mut data_structure = CommanderDataStructure::default();

        let req_lines = self.current_match.iter().filter(|x| {
            remove_chat_messages(x) && (x.contains(CHANGED_ROLE) || x.contains(DISCONNECTED))
        });

        for line in req_lines {
            if line.contains(DISCONNECTED) {
                let pattern_capture = player_disconnect.captures(line);
                let Some((_, [_, _, player_id, player_faction])) =
                    pattern_capture.map(|caps| caps.extract())
                else {
                    continue;
                };

                let faction_type = Game::get_factions(player_faction);
                if faction_type == Factions::Wildlife {
                    continue;
                }

                let player_id = player_id.parse::<i64>().unwrap_or_else(|e| {
                    panic!("Could not parse player id in player disconnect due to {e}")
                });

                if data_structure.is_current_commander(faction_type, player_id) {
                    let byte_matched_datetime_range = get_byte_indices(line, DATETIME_RANGE);

                    let time_end = match NaiveDateTime::parse_from_str(
                        line[byte_matched_datetime_range].trim(),
                        "%m/%d/%Y - %H:%M:%S",
                    ) {
                        Ok(time_end) => time_end,
                        Err(e) => {
                            panic!("Error in trying to parse round start time: {e}")
                        }
                    };
                    data_structure.add_commander_end(player_id, faction_type, time_end);
                }
            } else {
                let pattern_capture = role_change_pattern.captures(line);
                let Some((_, [player_name, _, player_id, player_faction, role])) =
                    pattern_capture.map(|caps| caps.extract())
                else {
                    continue;
                };
                let byte_matched_datetime_range = get_byte_indices(line, DATETIME_RANGE);

                let role_change_time = match NaiveDateTime::parse_from_str(
                    line[byte_matched_datetime_range].trim(),
                    "%m/%d/%Y - %H:%M:%S",
                ) {
                    Ok(datetime) => datetime,
                    Err(e) => {
                        panic!("Error in trying to parse round start time: {e}")
                    }
                };

                let faction_type = Game::get_factions(player_faction);
                if faction_type == Factions::Wildlife {
                    continue;
                }
                let player_id = player_id.parse::<i64>().unwrap_or_else(|e| {
                    panic!("error in parsing the commander player id due to : {e}")
                });

                self.players
                    .entry((player_id, faction_type))
                    .or_insert(Player::new(
                        player_id,
                        player_name.to_string(),
                        faction_type,
                        self.start_time,
                        self.end_time,
                        false,
                    ));

                if role == "Commander" {
                    data_structure.add_commander_start(player_id, faction_type, role_change_time);
                } else if data_structure.is_current_commander(faction_type, player_id) {
                    data_structure.add_commander_end(player_id, faction_type, role_change_time);
                }
            }
        }

        let mut to_change: Vec<(i64, Factions)> = Vec::new();
        for ((player_id, faction_type), _) in data_structure.current_commander.iter() {
            to_change.push((player_id.to_owned(), faction_type.to_owned()));
        }

        for (player_id, faction_type) in to_change {
            data_structure.add_commander_end(player_id, faction_type, self.end_time);
        }

        let final_commander = data_structure.get_all_commander();
        info!("The commanders are {final_commander:?}");

        for (faction, player_id) in final_commander {
            self.players
                .entry((player_id, faction))
                .and_modify(|e| e.set_commander());
        }
    }

    pub fn process_all_players(&mut self) {
        let joined_team_lines = self
            .current_match
            .iter()
            .filter(|x| remove_chat_messages(x))
            .filter(|x| x.contains(JOINED_TEAM) || x.contains(DISCONNECTED));

        let join_match_regex =
            Regex::new(r#""(.*?)<(.*?)><(.*?)><(.*?)>" joined team "(.*)""#).unwrap();

        let player_disconnect = Regex::new(r#""(.*?)<(.*?)><(.*?)><(.*?)>" disconnected"#).unwrap();

        for line in joined_team_lines {
            let joined_player = join_match_regex.captures(line.trim());

            if let Some((_, [player_name, _, player_id, _, player_faction])) =
                joined_player.map(|caps| caps.extract())
            {
                let faction_type = Game::get_factions(player_faction);

                if faction_type == Factions::Wildlife {
                    continue;
                }

                let player_id = player_id
                    .parse::<i64>()
                    .unwrap_or_else(|_| panic!("Error in parsing i64"));

                let byte_matched_datetime_range = get_byte_indices(line, DATETIME_RANGE);

                let start_time = match NaiveDateTime::parse_from_str(
                    line[byte_matched_datetime_range].trim(),
                    "%m/%d/%Y - %H:%M:%S",
                ) {
                    Ok(datetime) => datetime,
                    Err(e) => {
                        error!("Error in trying to parse round start time due to {e}");
                        panic!();
                    }
                };
                let player = self
                    .players
                    .entry((player_id, faction_type))
                    .or_insert_with(|| {
                        Player::new(
                            player_id,
                            player_name.to_string(),
                            faction_type,
                            start_time,
                            NaiveDateTime::MAX,
                            true,
                        )
                    });
                player.is_in_game = true;
                player.last_entered_time = start_time;
                player.last_left_time = NaiveDateTime::MAX;
            }

            let pattern_capture = player_disconnect.captures(line);
            if let Some((_, [player_name, _, player_id, player_faction])) =
                pattern_capture.map(|caps| caps.extract())
            {
                let faction_type = Game::get_factions(player_faction);

                if faction_type == Factions::Wildlife {
                    continue;
                }

                let player_id = player_id
                    .parse::<i64>()
                    .unwrap_or_else(|_| panic!("Error in parsing i64"));

                let byte_matched_datetime_range = get_byte_indices(line, DATETIME_RANGE);

                let disconnect_time = match NaiveDateTime::parse_from_str(
                    line[byte_matched_datetime_range].trim(),
                    "%m/%d/%Y - %H:%M:%S",
                ) {
                    Ok(datetime) => datetime,
                    Err(e) => {
                        error!("Error in trying to parse round start time due to {e}");
                        panic!();
                    }
                };
                let player = self
                    .players
                    .entry((player_id, faction_type))
                    .or_insert_with(|| {
                        Player::new(
                            player_id,
                            player_name.to_string(),
                            faction_type,
                            self.start_time,
                            disconnect_time,
                            false,
                        )
                    });
                player.is_in_game = false;
                player.last_left_time = disconnect_time;
                player.duration_played += player.last_left_time - player.last_entered_time;
            }
        }
    }

    pub fn get_current_match(&mut self, all_lines: &[PathBuf]) {
        let mut did_find_world_win = false;

        let mut current_match = Vec::new();

        for file in all_lines.iter().rev() {
            let reader = match File::open(file) {
                Ok(open_file) => RevLines::new(open_file),
                Err(e) => panic!("Error in opening the log file due to: {e}"),
            };

            for option_line in reader {
                let line = match option_line {
                    Ok(line) => line,
                    Err(e) => {
                        warn!("Cannot read line due to {e}");
                        continue;
                    }
                };
                let byte_matched_round_end_range = get_byte_indices(&line, ROUND_END_RANGE);
                let byte_matched_round_start_range = get_byte_indices(&line, ROUND_START_RANGE);
                let byte_matched_datetime_range = get_byte_indices(&line, DATETIME_RANGE);
                if line[byte_matched_round_end_range].trim() == ROUND_END {
                    self.end_time = match NaiveDateTime::parse_from_str(
                        line[byte_matched_datetime_range].trim(),
                        "%m/%d/%Y - %H:%M:%S",
                    ) {
                        Ok(datetime) => datetime,
                        Err(e) => {
                            error!("Error in trying to parse round start time due to {e}");
                            panic!()
                        }
                    };
                    did_find_world_win = true;
                    current_match.push(line);
                } else if did_find_world_win {
                    current_match.push(line.clone());
                    if line[byte_matched_round_start_range].trim() == ROUND_START {
                        self.start_time = match NaiveDateTime::parse_from_str(
                            line[DATETIME_RANGE].trim(),
                            "%m/%d/%Y - %H:%M:%S",
                        ) {
                            Ok(datetime) => datetime,
                            Err(e) => {
                                error!("Error in trying to parse round start time due to {e}");
                                panic!()
                            }
                        };
                        current_match.reverse();
                        self.current_match = current_match;
                        return;
                    }
                }
            }
        }
    }

    pub fn get_match_type(&mut self) {
        let match_type_thing = self.current_match[0][MATCH_TYPE_RANGE..].trim();
        let match_type_regex = Regex::new(r#"\(gametype "(.*?)"\)"#).unwrap();
        //let match_type = match_type_regex.find(match_type_thing).unwrap().as_str();
        let match_type = match_type_regex
            .captures(match_type_thing)
            .unwrap()
            .get(1)
            .unwrap_or_else(|| panic!("Couldn't parse the match_type"))
            .as_str();

        if match_type == SOL_VS_ALIEN {
            self.match_type = Modes::SolVsAlien
        } else if match_type == CENTAURI_VS_SOL {
            self.match_type = Modes::CentauriVsSol
        } else if match_type == CENTAURI_VS_SOL_VS_ALIEN {
            self.match_type = Modes::CentauriVsSolVsAlien
        }
    }

    pub fn process_kills(&mut self) {
        let kill_regex = match Regex::new(
            r#""(.*?)<(.*?)><(.*?)><(.*?)>" killed "(.*?)<(.*?)><(.*?)><(.*?)>" with "(.*)" \(dmgtype "(.*)"\) \(victim "(.*)"\)"#,
        ) {
            Ok(kill_regex) => kill_regex,
            Err(e) => panic!("Error in creating the kill regex: {e}"),
        };

        for kill_line in &self.current_match {
            if !kill_line.contains(KILLED) {
                continue;
            }

            let kill_matches = kill_regex.captures(kill_line);
            let Some((
                _,
                [player_name, _, player_id, player_faction, enemy_name, _, enemy_id, enemy_faction, _, _, victim],
            )) = kill_matches.map(|cap| cap.extract())
            else {
                continue;
            };

            if let Ok(player_id) = player_id.parse::<i64>() {
                let faction_type = Game::get_factions(player_faction);
                if faction_type == Factions::Wildlife {
                    continue;
                }
                let enemy_faction_type = Game::get_factions(enemy_faction);
                let player = self
                    .players
                    .entry((player_id, faction_type))
                    .or_insert_with(|| {
                        Player::new(
                            player_id,
                            player_name.to_string(),
                            faction_type,
                            self.start_time,
                            self.end_time,
                            false,
                        )
                    });
                player.update_unit_kill(victim, enemy_faction_type);
            };

            if let Ok(enemy_id) = enemy_id.parse::<i64>() {
                let enemy_faction_type = Game::get_factions(enemy_faction);
                if enemy_faction_type == Factions::Wildlife {
                    continue;
                }
                let enemy_player = self
                    .players
                    .entry((enemy_id, enemy_faction_type))
                    .or_insert_with(|| {
                        Player::new(
                            enemy_id,
                            enemy_name.to_string(),
                            enemy_faction_type,
                            self.start_time,
                            self.end_time,
                            false,
                        )
                    });
                enemy_player.update_death(victim);
            };
        }
    }

    pub fn process_structure_kills(&mut self) {
        let kill_regex = match Regex::new(
            r#""(.*?)<(.*?)><(.*?)><(.*?)>" triggered "structure_kill" \(structure "(.*)"\) \(struct_team "(.*)"\)"#,
        ) {
            Ok(kill_regex) => kill_regex,
            Err(e) => panic!("Error in creating the kill regex: {e}"),
        };

        for kill_line in &self.current_match {
            if !kill_line.contains(STRUCTURE_KILL) {
                continue;
            }
            let kill_matches = kill_regex.captures(kill_line);
            let Some((_, [player_name, _, player_id, player_faction, enemy_structure, _])) =
                kill_matches.map(|cap| cap.extract())
            else {
                continue;
            };

            let faction_type = Game::get_factions(player_faction);

            if faction_type == Factions::Wildlife {
                continue;
            }

            match player_id.parse::<i64>() {
                Ok(player_id) => {
                    //NOTE Why should player not be specified as mut here?
                    let player = self
                        .players
                        .entry((player_id, faction_type))
                        .or_insert_with(|| {
                            Player::new(
                                player_id,
                                player_name.to_string(),
                                faction_type,
                                self.start_time,
                                self.end_time,
                                false,
                            )
                        });
                    player.update_structure_kill(enemy_structure);
                }
                Err(_) => {
                    info!("Couldn't parse the player_id. Most likely AI");
                }
            };
        }
    }

    pub fn get_current_map(&mut self, all_lines: &[PathBuf]) {
        let map_regex = match Regex::new(r#"Loading map "(.*)""#) {
            Ok(map_regex) => map_regex,
            Err(_) => {
                error!("Error in creating the get_current_map_regex");
                panic!();
            }
        };

        let mut files_read = Vec::new();

        for file in all_lines.iter().rev() {
            files_read.push(file);
            let reader = match File::open(file) {
                Ok(open_file) => RevLines::new(open_file),
                Err(e) => {
                    error!("Error in opening the log file due to: {e}");
                    panic!();
                }
            };

            for option_line in reader {
                let line = match option_line {
                    Ok(line) => {
                        if !line.contains(LOADING_MAP) {
                            continue;
                        }
                        line
                    }
                    Err(e) => {
                        warn!("Cannot read line due to {e}");
                        continue;
                    }
                };
                let map_matched = map_regex.captures(&line);
                match map_matched {
                    Some(map) => {
                        let map_str = map.get(1).unwrap().as_str();
                        if map_str == "NarakaCity" {
                            self.map = Maps::NarakaCity;
                        } else if map_str == "MonumentValley" {
                            self.map = Maps::MonumentValley;
                        } else if map_str == "RiftBasin" {
                            self.map = Maps::RiftBasin;
                        } else if map_str == "Badlands" {
                            self.map = Maps::Badlands;
                        } else if map_str == "GreatErg" {
                            self.map = Maps::GreatErg;
                        } else if map_str == "TheMaw" {
                            self.map = Maps::TheMaw;
                        } else if map_str == "CrimsonPeak" {
                            self.map = Maps::CrimsonPeak;
                        } else if map_str == "NorthPolarCap" {
                            self.map = Maps::NorthPolarCap;
                        } else {
                            error!("Map {map_str} not found. Exiting parsing.");
                            panic!();
                        }

                        info!("Files read for finding the current map are {files_read:?}");
                        return;
                    }
                    None => continue,
                }
            }
        }
    }

    pub fn get_winning_team(&mut self) {
        let winning_team_log = self
            .current_match
            .iter()
            .rev()
            .filter(|x| x.contains(TRIGGERED));

        let victory_regex = match Regex::new(r#"Team "(.*?)" triggered "Victory""#) {
            Ok(map_regex) => map_regex,
            Err(e) => {
                error!("Error in creating the get_current_map_regex due to: {e}");
                panic!()
            }
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

fn _remove_date_data(line: &str) -> &str {
    if line.len() > DATETIME_END {
        let byte_corrected_datetimeend = get_byte_indices(line, DATETIME_END..line.len());
        &line.trim()[byte_corrected_datetimeend]
    } else {
        ""
    }
}

fn parse_info(all_lines: Vec<PathBuf>) -> Game {
    let mut game = Game::default();
    //NOTE Possible to parallelize them, but probably not worth it.
    game.get_current_map(&all_lines);
    info!("current map is {:?}", game.map);
    game.get_current_match(&all_lines);
    game.get_match_type();
    game.get_winning_team();
    game.process_all_players();
    game.process_kills();
    game.process_structure_kills();
    game.get_commanders();
    game.process_player_durations();
    game
}

pub fn checking_folder(path: &Path) -> Game {
    info!("The path of the folder is {path:?}");
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

    info!("Parsing the file");

    parse_info(log_files)
}

pub fn checking_file(path: &Path) -> Game {
    info!("The path of the folder is {path:?}");
    info!("Parsing the file");
    parse_info(vec![path.to_path_buf()])
}
