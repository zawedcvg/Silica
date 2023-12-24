from enum import Enum
import sys
import math
import cProfile
import collections
from datetime import datetime
import re

class Factions(Enum):
    SOL = "Human (Sol)"
    CENTAURI = "Human (Centauri)"
    ALIEN = "Alien"

class Modes(Enum):
    SOL_VS_ALIEN = "HUMANS_VS_ALIENS"
    # CENTAURI_VS_ALIEN = 1
    CENTAURI_VS_SOL = "HUMANS_VS_HUMANS"
    # CENTAURI_VS_CENTAURI = 3
    # SOL_VS_SOL = 4
    # ALIENS_VS_ALIEN = 5
    CENTAURI_VS_SOL_VS_ALIEN = "HUMANS_VS_HUMANS_VS_ALIENS" 

class Player:
    def __init__(self, player_id: int, player_name: str, faction_type: Factions):
        self.player_id = player_id
        self.player_name = player_name
        self.faction_type = faction_type
        #going to use both is not commander and 0 kills for checking if there is a fps side?
        self.is_commander = False
        self.unit_kill = 0
        self.structure_kill = 0
        self.death = 0
        #allocate method for points
        self.points = 0
        self.winner = False

    def update_structure_kill(self, structure):
        self.structure_kill += 1
    def update_unit_kill(self, unit):
        self.unit_kill += 1
    def update_death(self):
        self.death += 1
        #TODO subtract from points
    def set_commander(self):
        self.is_commander = True
    def is_fps(self):
        return not self.is_commander
    def did_win(self, winning_team: Factions):
        self.winner = (winning_team == self.faction_type)

    def __str__(self):
        return f"name: {self.player_name}, id: {self.player_id}, faction_type: {self.faction_type.value}, \
unit_kills: {self.unit_kill}, structure_kill: {self.structure_kill}, deaths = {self.death} self.winner = {self.winner} is_commander = {self.is_commander}"

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


def is_valid_faction_type(match_type: Modes, faction_type: Factions):
    if match_type == Modes.CENTAURI_VS_SOL:
        return faction_type != Factions.ALIEN
    elif match_type == Modes.SOL_VS_ALIEN:
        return faction_type != Factions.CENTAURI
    elif match_type == Modes.CENTAURI_VS_SOL_VS_ALIEN:
        return True

def get_current_match(all_lines):
    inverted_list = all_lines[::-1]
    for i, value in enumerate(inverted_list):
        if value[25:54].strip() == ROUND_START:
            return all_lines[len(all_lines) - i - 1:]

def is_current_match_completed(match_info):
    for i in match_info[::-1]:
        if i[25:52].strip() == ROUND_END:
            date_string = i[1:23].strip()
            global END_TIME
            END_TIME = datetime.strptime(date_string, "%m-%d-%Y - %H:%M:%S")
            return True
    return False

def get_match_type(match_details):
    #TODO add method for getting the type
    match_type_pattern = r'\(gametype "(.*?)"\)'
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
    return game_mode, factions

def get_commanders(match_log_info, match_mode_info, all_players):
    #TODO complete the implementation once logging bug is fixed
    #get the person with the max time as commander
    commander_joined_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" changed role to "Commander"'
    commander_left_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" changed role to "Infantry"'
    #mode, factions = match_mode_info incase i add more checks
    all_commanders = collections.defaultdict(list)
    commander_durations = collections.defaultdict(list)
    commander_log_info = filter(lambda x: "changed" in x, match_log_info)
    #TODO, fix to set, else leave it as is.
    for i in commander_log_info:
        joined_match = re.search(commander_joined_pattern, i)
        left_match = re.search(commander_left_pattern, i)
        if joined_match:
            date_string = i[1:23].strip()
            start_time = datetime.strptime(date_string, "%m-%d-%Y - %H:%M:%S")
            commander = int(joined_match.group(3))
            faction_type = Factions(joined_match.group(4))
            player = all_players[(commander, faction_type)]
            #all_commanders is the all the factions and all the commanders they have had
            all_commanders[faction_type].append(player)
            commander_durations[player].append(start_time)
        #change this once logging mod updates
        elif left_match:
            commander = int(left_match.group(3))
            faction_type = Factions(left_match.group(4))
            player = all_players[(commander, faction_type)]
            if player not in all_commanders[faction_type]:
                continue
            date_string = i[1:23].strip()
            end_time = datetime.strptime(date_string, "%m-%d-%Y - %H:%M:%S")
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

        faction_commander.set_commander()

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
    return all_players

def process_structure_kills(all_match_info, all_players):
    structure_kill_info = filter(lambda x: STRUCTURE_KILL in x.split(" "), all_match_info)
    structure_killed_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" triggered "structure_kill" \(structure "(.*)"\) \(struct_team "(.*)"\)'
    for i in structure_kill_info:
        match = re.search(structure_killed_pattern, i.strip())
        if match:
            player_id = int(match.group(3))
            player_faction = match.group(4)
            structure = match.group(5)
            all_players[(player_id, Factions(player_faction))].update_structure_kill(structure)

def process_unit_kills(all_match_info, all_players):
    unit_kill_info = filter(lambda x: KILLED in x.split(" "), all_match_info)
    unit_kill_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" killed "(.*?)<(.*?)><(.*?)><(.*?)>" with "(.*)"'
    # for i in
    for i in unit_kill_info:
        match = re.search(unit_kill_pattern, i.strip())
        if match:
            player_id = (match.group(3))
            #TODO add victim things
            player_faction = match.group(4)
            enemy_name = match.group(5)
            enemy_id = match.group(7)
            enemy_faction = match.group(8)
            if enemy_id != "":
                all_players[(int(enemy_id), Factions(enemy_faction))].update_death()
            if player_id != "":
                all_players[(int(player_id), Factions(player_faction))].update_unit_kill(enemy_name)#change/fix this

def probability(rating1, rating2):
    return 1.0 * 1.0 / (1 + 1.0 * math.pow(10, 1.0 * (rating1 - rating2) / 400))

def elo_rating_fps(Ra, Rb, K, d):
    #TODO
    #calculate based on points. use maybe an exponential thing?
    pass

#https://www.geeksforgeeks.org/elo-rating-algorithm/
def elo_rating_commander(Ra, Rb, K, d):
    # To calculate the Winning
    # Probability of Player B
    Pb = probability(Ra, Rb)

    # To calculate the Winning
    # Probability of Player A
    Pa = probability(Rb, Ra)

    # Case -1 When Player A wins
    # Updating the Elo Ratings
    if (d == 1):
        Ra = Ra + K * (1 - Pa)
        Rb = Rb + K * (0 - Pb)

    # Case -2 When Player B wins
    # Updating the Elo Ratings
    else:
        Ra = Ra + K * (0 - Pa)
        Rb = Rb + K * (1 - Pb)

    print("Updated Ratings:-")
    print("Ra =", round(Ra, 6), " Rb =", round(Rb, 6))

#reading the file
#TODO add flag to parse the entire log as for testing/some purpose

def checking(file_name):
    file_pointer = open(file_name, "r")
    all_lines = file_pointer.readlines()
    match_log_info = get_current_match(all_lines)
    is_complete = is_current_match_completed(match_log_info)
    if not is_complete:
        print("Aborting parsing, last match has incomplete information")
        exit()
    all_essential_info = list(filter(remove_chat_messages, match_log_info))
    winning_team = get_winning_team(all_essential_info)
    all_essential_info = [remove_date_data(line) for line in all_essential_info]
    all_players = get_all_players(all_essential_info, winning_team)
    process_structure_kills(all_essential_info, all_players)
    process_unit_kills(all_essential_info, all_players)
    get_commanders(match_log_info, None, all_players)
    for i in all_players.values():
        print(i)

if __name__ == "__main__":
    checking(sys.argv[1])

# checking()
# cProfile.run('checking()')
