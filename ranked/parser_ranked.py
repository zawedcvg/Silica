from enum import Enum
import os
import sys
import math
import cProfile
import collections
from datetime import datetime
import re

class Factions(Enum):
    SOL = "Sol"
    CENTAURI = "Centauri"
    ALIEN = "Alien"
    WILDLIFE = "Wildlife"

class Modes(Enum):
    SOL_VS_ALIEN = "HUMANS_VS_ALIENS"
    CENTAURI_VS_SOL = "HUMANS_VS_HUMANS"
    CENTAURI_VS_SOL_VS_ALIEN = "HUMANS_VS_HUMANS_VS_ALIENS" 

class Player:
    def __init__(self, player_id: int, player_name: str, faction_type: Factions):
        self.player_id = player_id
        self.player_name = player_name
        self.faction_type = faction_type
        #going to use both is not commander and 0 kills for checking if there is a fps side?
        self.is_commander = False
        self.unit_kill = [0, 0, 0]
        self.total_unit_kills = 0
        # self.tier_one
        self.total_structure_kills = 0
        self.structure_kill = [0, 0, 0]
        # self.tier_one_structure_kills = 
        self.death = 0
        #allocate method for points
        self.points = 0
        self.winner = False

    def update_structure_kill(self, structure):
        self.total_structure_kills += 1
        if structure in TIER_ONE_STRUCTURES:
            self.structure_kill[0] += 1
            self.points += 10
        elif structure in TIER_TWO_STRUCTURES:
            self.structure_kill[1] += 1
            self.points += 50
        elif structure in TIER_THREE_STRUCTURES:
            self.structure_kill[2] += 1
            self.points += 100
        else:
            print(structure)

    def update_unit_kill(self, unit):
        self.total_unit_kills += 1
        if unit in TIER_ONE_UNITS:
            self.unit_kill[0] += 1
            self.points += 1
        elif unit in TIER_TWO_UNITS:
            self.unit_kill[1] += 1
            self.points += 10
        elif unit in TIER_THREE_UNITS:
            self.unit_kill[2] += 1
            if unit == "Queen":
                self.points += 100
            else:
                self.points += 50
        else:
            print(unit)
    def update_death(self, unit):
        self.death += 1
        if unit in TIER_ONE_UNITS:
            self.points -= 1
        elif unit in TIER_TWO_UNITS:
            self.points -= 10
        elif unit in TIER_THREE_UNITS:
            self.points -= 50
        #TODO subtract from points
    def set_commander(self):
        self.is_commander = True
    def is_fps(self):
        all_sum = sum(self.unit_kill) + sum(self.structure_kill) + self.death
        return all_sum != 0
    def did_win(self, winning_team: Factions):
        self.winner = (winning_team == self.faction_type)
    def __str__(self):
        return f"name: {self.player_name}, id: {self.player_id}, faction_type: {self.faction_type.value}, \
unit_kills: {self.unit_kill}, structure_kill: {self.structure_kill}, deaths = {self.death} self.winner = {self.winner} is_infantry = {self.is_fps()} is_commander = {self.is_commander} points= {self.points}"

#TODO
#to get thing, just to Factions(name).name?

#CONSTANTS
STRUCTURE_KILL = "\"structure_kill\""
KILLED = "killed"
JOINED_TEAM = "joined team"
CHAT = "say"
TEAM_CHAT = "say_team"
ROUND_START = "World triggered \"Round_Start\""
ROUND_END = "World triggered \"Round_Win\""
END_TIME = ""
START_TIME = ""
LOADING_MAP = "Loading map"


TIER_ONE_UNITS = ["Crab", "Crab_Horned", "Shocker", "Shrimp", "Soldier_Rifleman", "Soldier_Scout", "Soldier_Heavy", "Soldier_Marksman", "LightQuad", "Wasp", "HoverBike", "Worm", "FlakTruck"]

TIER_TWO_UNITS = ["Behemoth", "Hunter", "LightArmoredCar", "ArmedTransport", "HeavyArmoredCar", "TroopTransport", "HeavyQuad", "RocketLauncher", "PulseTank", "AirGunship", "AirFighter", "AirDropship", "Dragonfly", "Firebug", "Soldier_Commando", "GreatWorm"]

TIER_THREE_UNITS = ["Queen", "Scorpion", "Goliath", "BomberCraft", "HeavyHarvester", "HoverTank", "RailgunTank", "SiegeTank", "AirBomber", "Defiler", "Colossus"]

TIER_ONE_STRUCTURES = ["HiveSpire", "LesserSpawningCyst", "Node", "ThornSpire", "Outpost", "RadarStation", "Silo"]

TIER_TWO_STRUCTURES = ["BioCache", "Barracks", "HeavyVehicleFactory", "LightVehicleFactory", "QuantumCortex", "GreaterSpawningCyst", "Refinery", "Bunker", "ConstructionSite_TurretHeavy", "TurretHeavy", "TurretMedium", "GrandSpawningCyst", "TurretAARocket"]

TIER_THREE_STRUCTURES = ["ResearchFacility", "Nest", "UltraHeavyVehicleFactory", "Headquarters", "GrandSpawningCyst", "ColossalSpawningCyst", "AirFactory"]

def is_valid_faction_type(match_type: Modes, faction_type: Factions):
    if match_type == Modes.CENTAURI_VS_SOL:
        return faction_type != Factions.ALIEN
    elif match_type == Modes.SOL_VS_ALIEN:
        return faction_type != Factions.CENTAURI
    elif match_type == Modes.CENTAURI_VS_SOL_VS_ALIEN:
        return True

def create_new_player(all_players, match_info, player_id, player_faction, player_name):
    player_id = int(player_id)
    new_player = Player(player_id, player_name, Factions(player_faction))
    winning_team = get_winning_team(match_info)
    new_player.did_win(winning_team)
    all_players[(player_id, Factions(player_faction))] = new_player

def get_match_start(all_lines):
    for i, value in enumerate(all_lines):
        if value[25:54].strip() == ROUND_START:
            global START_TIME
            date_string = value[1:23].strip()
            START_TIME = datetime.strptime(date_string, "%m/%d/%Y - %H:%M:%S")

def get_current_match(all_lines):
    inverted_list = all_lines[::-1]
    for i, value in enumerate(inverted_list):
        if value[25:54].strip() == ROUND_START:
            global START_TIME
            date_string = value[1:23].strip()
            START_TIME = datetime.strptime(date_string, "%m/%d/%Y - %H:%M:%S")
            return all_lines[len(all_lines) - i - 1:]

def get_latest_complete_match(all_lines):
    inverted_list = all_lines[::-1]
    did_find_world_win = False
    end_index = None
    for i, value in enumerate(inverted_list):
        if value[25:52].strip() == ROUND_END:
            did_find_world_win = True
            end_index = len(all_lines) - i
        elif value[25:54].strip() == ROUND_START and did_find_world_win:
            global START_TIME
            date_string = value[1:23].strip()
            START_TIME = datetime.strptime(date_string, "%m/%d/%Y - %H:%M:%S")
            return all_lines[len(all_lines) - i - 1: end_index]
# def 
def get_all_matches(all_lines):
    # for i in all_lines:
    start = []
    end = []
    for i, value in enumerate(all_lines):
        if value[25:54].strip() == ROUND_START:
            start.append(i)
        elif value[25:52].strip() == ROUND_END:
            end.append(i)

    if start[0] > end[0]:
        end.pop(0)

    return list(zip(start, end))

    # exit()


def is_current_match_completed(match_info):
    for i in match_info[::-1]:
        if i[25:52].strip() == ROUND_END:
            date_string = i[1:23].strip()
            global END_TIME
            END_TIME = datetime.strptime(date_string, "%m/%d/%Y - %H:%M:%S")
            return True
    return False

def filter_irrelevant_fps_players():
    pass


def get_match_type(match_details):
    """Gets the game mode, factions played and the number of seconds the game lasted"""
    match_type_pattern = r'\(gametype "(.*?)"\)'
    global START_TIME
    global END_TIME
    match_type_info = match_details[0][54:].strip()
    match_type = re.search(match_type_pattern, match_type_info)
    if match_type is None:
        print("Incorrect format of match type")
        exit()
    game_mode = Modes(match_type.group(1))
    if game_mode == Modes.CENTAURI_VS_SOL:
        factions = [Factions.SOL, Factions.CENTAURI]
    elif game_mode == Modes.CENTAURI_VS_SOL_VS_ALIEN:
        factions = [Factions.SOL, Factions.CENTAURI, Factions.ALIEN]
    elif game_mode == Modes.SOL_VS_ALIEN:
        factions = [Factions.SOL, Factions.ALIEN]
    return game_mode, factions, (END_TIME - START_TIME).total_seconds()

def get_commanders(match_log_info, match_mode_info, all_players):
    #TODO Refactor this to improve readability
    #get the person with the max time as commander
    commander_joined_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" changed role to "Commander"'
    # commander_joined_pattern = r'".*?<.*?><.*?><.*?>" say "<b><color=#.*?>\[.*?\]</b> Promoted <.*?>(.*?)<color=.*?> to commander for '
    commander_left_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" changed role to "Infantry"'
    commander_left_pattern_2 = r'"(.*?)<(.*?)><(.*?)><(.*?)>" disconnected'
    current_commander = collections.defaultdict(lambda: 0)
    # commander_left_pattern = r'".*?<.*?><.*?><.*?>" say "<b><color=#.*?>\[.*?\]</b> <.*?>(.*?)<color=.*?> left commander position vacant for '
    #mode, factions = match_mode_info incase i add more checks
    all_commanders = collections.defaultdict(list)
    commander_durations = collections.defaultdict(list)
    commander_log_info = filter(lambda x: "changed" in x or "disconnected" in x, match_log_info)
    #TODO, fix to set, else leave it as is.
    for i in commander_log_info:
        joined_match = re.search(commander_joined_pattern, i)
        left_match = re.search(commander_left_pattern, i)
        left_match_2 = re.search(commander_left_pattern_2, i)
        if joined_match:
            date_string = i[1:23].strip()
            start_time = datetime.strptime(date_string, "%m/%d/%Y - %H:%M:%S")
            commander = int(joined_match.group(3))
            faction_type = Factions(joined_match.group(4))
            current_commander[faction_type] = commander
            if (commander, faction_type) not in all_players:
                create_new_player(all_players, match_log_info, commander, joined_match.group(4), joined_match.group(1))

            player = all_players[(commander, faction_type)]
            #all_commanders is the all the factions and all the commanders they have had
            all_commanders[faction_type].append(player)
            commander_durations[player].append(start_time)
        #change this once logging mod updates
        elif left_match:
            commander = int(left_match.group(3))
            faction_type = Factions(left_match.group(4))
            current_commander[faction_type] = 0
            if (commander, faction_type) not in all_players:
                create_new_player(all_players, match_log_info, commander, left_match.group(4), left_match.group(1))
            player = all_players[(commander, faction_type)]
            if player not in all_commanders[faction_type]:
                continue
            date_string = i[1:23].strip()
            end_time = datetime.strptime(date_string, "%m/%d/%Y - %H:%M:%S")
            commander_durations[player].append(end_time)
        elif left_match_2:
            commander = int(left_match_2.group(3))
            faction = left_match_2.group(4)
            if faction == "":
                continue
            faction_type = Factions(faction)
            if current_commander[faction_type] != commander:
                continue
            current_commander[faction_type] = 0
            if (commander, faction_type) not in all_players:
                create_new_player(all_players, match_log_info, commander, left_match_2.group(4), left_match_2.group(1))
            player = all_players[(commander, faction_type)]
            if player not in all_commanders[faction_type]:
                continue
            date_string = i[1:23].strip()
            end_time = datetime.strptime(date_string, "%m/%d/%Y - %H:%M:%S")
            commander_durations[player].append(end_time)
    for duration in commander_durations.values():
        if len(duration) % 2 != 0:
            duration.append(END_TIME)
    final_commander = {}
    for faction, commanders in all_commanders.items():
        max_duration = -1
        faction_commander = None
        for commander in commanders:
            commander_duration = commander_durations[commander]
            total_commander_duration = sum([(commander_duration[i + 1] - commander_duration[i]).total_seconds()
                       for i in range(0, len(commander_duration), 2)])
            if total_commander_duration > max_duration:
                max_duration = total_commander_duration
                faction_commander = commander
        final_commander[faction] = faction_commander

        if faction_commander is not None:
            faction_commander.set_commander()
        else:
            print(f"Error occured while parsing faction_commander. faction_commander was None")
    return final_commander

def remove_chat_messages(line):
    words = line.split(" ")
    return CHAT not in words and TEAM_CHAT not in words

def remove_date_data(line):
    return line.strip()[25:]

def get_structure_killed_filter(all_req):
    return filter(lambda x: STRUCTURE_KILL in x.split(" "), all_req)

def get_kills(all_req):
    return filter(lambda x: KILLED in x.split(" "), all_req)

def get_winning_team(all_req):
    winning_team_log = filter(lambda x: "triggered" in x, all_req[::-1])
    pattern = r'Team "(.*?)" triggered "Victory"'
    for i in winning_team_log:
        match = re.search(pattern, i)
        if match:
            return Factions(match.group(1))


def get_all_players(all_req, winning_team):
    #what happens during balance?
    join_info = filter(lambda x: JOINED_TEAM in x, all_req)
    join_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" joined team "(.*)"'
    all_players = {}
    for i in join_info:
        match = re.search(join_pattern, i.strip())
        if match:
            player_name = match.group(1)
            player_id = int(match.group(3))
            player_faction = match.group(5)
            new_player = Player(player_id, player_name, Factions(player_faction))
            new_player.did_win(winning_team)
            all_players[(player_id, Factions(player_faction))] = new_player
        # for in in all_req:

    return all_players

def process_structure_kills(all_match_info, all_players):
    structure_kill_info = filter(lambda x: STRUCTURE_KILL in x.split(" "), all_match_info)
    structure_killed_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" triggered "structure_kill" \(structure "(.*)"\) \(struct_team "(.*)"\)'
    for i in structure_kill_info:
        match = re.search(structure_killed_pattern, i.strip())
        if match:
            player_name = match.group(1)
            player_id = int(match.group(3))
            player_faction = match.group(4)
            structure = match.group(5)
            if player_id != "":
                # if Factions(plaue)
                if Factions(player_faction) == Factions.WILDLIFE:
                    continue
                if (int(player_id), Factions(player_faction)) not in all_players:
                    create_new_player(all_players, all_match_info, player_id, player_faction, player_name)
                all_players[(player_id, Factions(player_faction))].update_structure_kill(structure)

def process_unit_kills(all_match_info, all_players):
    unit_kill_info = filter(lambda x: KILLED in x.split(" "), all_match_info)
    unit_kill_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" killed "(.*?)<(.*?)><(.*?)><(.*?)>" with "(.*)" \(dmgtype "(.*)"\) \(victim "(.*)"\)'
    # for i in
    for i in unit_kill_info:
        match = re.search(unit_kill_pattern, i.strip())
        if match:
            player_name = match.group(1)
            player_id = (match.group(3))
            #TODO add victim things
            player_faction = match.group(4)
            enemy_name = match.group(5)
            enemy_id = match.group(7)
            enemy_faction = match.group(8)
            victim = match.group(11)
            if enemy_id != "":
                if Factions(enemy_faction) != Factions.WILDLIFE:
                    if (int(enemy_id), Factions(enemy_faction)) not in all_players:
                        create_new_player(all_players, all_match_info, enemy_id, enemy_faction, enemy_name)
                    all_players[(int(enemy_id), Factions(enemy_faction))].update_death(victim)
            if player_id != "":
                if Factions(player_faction) == Factions.WILDLIFE:
                    continue
                if (int(player_id), Factions(player_faction)) not in all_players:
                    create_new_player(all_players, all_match_info, player_id, player_faction, player_name)
                all_players[(int(player_id), Factions(player_faction))].update_unit_kill(victim)#change/fix this for friendly kills?

def probability(rating1, rating2):
    return 1.0 * 1.0 / (1 + 1.0 * math.pow(10, 1.0 * (rating1 - rating2) / 400))

#https://www.geeksforgeeks.org/elo-rating-algorithm/
def elo_rating_commander(elo_list, win_list, K=30):
    if len(elo_list) == 0:
        return []
    if len(elo_list) == 1:
        Ra = elo_list[0]
        Rb = 1000
        Pb = probability(Ra, Rb)

        # To calculate the Winning
        # Probability of Player A
        Pa = probability(Rb, Ra)

        # Case -1 When Player A wins
        # Updating the Elo Ratings
        if (win_list[0]):
            Ra = Ra + K * (1 - Pa)
            Rb = Rb + K * (0 - Pb)

        # Case -2 When Player B wins
        # Updating the Elo Ratings
        else:
            Ra = Ra + K * (0 - Pa)
            Rb = Rb + K * (1 - Pb)

        print("Updated Ratings:-")
        print("Ra =", round(Ra, 6))
        return [int(Ra)]

    if len(elo_list) == 2:
        Ra, Rb = elo_list
        Pb = probability(Ra, Rb)

        # To calculate the Winning
        # Probability of Player A
        Pa = probability(Rb, Ra)

        # Case -1 When Player A wins
        # Updating the Elo Ratings
        if (win_list[0]):
            Ra = Ra + K * (1 - Pa)
            Rb = Rb + K * (0 - Pb)

        # Case -2 When Player B wins
        # Updating the Elo Ratings
        else:
            Ra = Ra + K * (0 - Pa)
            Rb = Rb + K * (1 - Pb)

        print("Updated Ratings:-")
        print("Ra =", round(Ra, 6), " Rb =", round(Rb, 6))
        return int(Ra), int(Rb)
    else:
        Ra, Rb, Rc = elo_list
        P = []
        P.append(probability(Ra, Rb) + probability(Ra, Rc))
        P.append(probability(Rb, Ra) + probability(Rb, Rc))
        P.append(probability(Rc, Ra) + probability(Rc, Rb))

        R = []
        for p, w, r in zip(P, win_list, elo_list):
            thing = 1 if w else 0
            new_R = r + K * 2 * (thing - p/6)
            R.append(int(new_R))
        # Ra = Ra + Pa * ()
        return R


def get_current_map(all_file_info):
    all_essential_info = list(filter(remove_chat_messages, all_file_info))
    all_essential_info = [remove_date_data(line) for line in all_essential_info]
    all_essential_info = filter(lambda x: LOADING_MAP in x, all_essential_info)
    current_map_info = list(all_essential_info)[-1]
    loading_map_pattern = r'Loading map "(.*)"'
    map_search = re.search(loading_map_pattern, current_map_info)
    if map_search == None:
        print("Map info not found in the current folder")
        return None
    else:
        return map_search.group(1)


#reading the file
def checking_all(file_name):
    file_pointer = open(file_name, 'r', errors='replace')
    all_lines = file_pointer.readlines()
    # match_log_info = get_current_match(all_lines)
    match_log_info = get_all_matches(all_lines)
    all_parse_info = []

    for start, end in match_log_info:
        the_match_lines = all_lines[start:end + 1]
        all_parse_info.append(parse_info(the_match_lines))

    return all_parse_info

def parse_info(match_log_info):
    """Parses a list of strings to extract the match type, contributions by players and commanders,
    and the winning team  """
    get_match_start(match_log_info)
    is_complete = is_current_match_completed(match_log_info)
    if not is_complete:
        print("Aborting parsing, last match has incomplete information")
        exit()
    match_type_info = (get_match_type(match_log_info))
    all_essential_info = list(filter(remove_chat_messages, match_log_info))
    winning_team = get_winning_team(all_essential_info)
    all_essential_info = [remove_date_data(line) for line in all_essential_info]
    all_players = get_all_players(all_essential_info, winning_team)
    process_structure_kills(all_essential_info, all_players)
    process_unit_kills(all_essential_info, all_players)

    _ = get_commanders(match_log_info, None, all_players)
    return match_type_info, winning_team, all_players



def checking_folder(folder_name):
    files = [i for i in os.listdir(folder_name) if i.endswith(".log")]
    files = [os.path.join(folder_name, i) for i in os.listdir(folder_name) if i.endswith(".log")]
    files = sorted(files)
    all_lines = []

    for file in files:
        file_pointer = open(file, "r", errors='replace')
        all_lines += file_pointer.readlines()
        file_pointer.close()
    match_log_info = get_latest_complete_match(all_lines)
    current_map = get_current_map(all_lines)
    return *parse_info(match_log_info), current_map

def checking(file_name):
    file_pointer = open(file_name, "r", errors='replace')
    all_lines = file_pointer.readlines()
    match_log_info = get_latest_complete_match(all_lines)
    return parse_info(match_log_info)

if __name__ == "__main__":
    # checking_all(sys.argv[1])
    print(checking_folder(sys.argv[1]))
