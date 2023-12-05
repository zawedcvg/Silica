from enum import Enum
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
    SOL_VS_ALIEN = 0
    CENTAURI_VS_ALIEN = 1
    CENTAURI_VS_SOL = 2
    CENTAURI_VS_CENTAURI = 3
    SOL_VS_SOL = 4
    ALIENS_VS_ALIEN = 5
    CENTAURI_VS_SOL_VS_ALIEN = 6

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
unit_kills: {self.unit_kill}, structure_kill: {self.structure_kill}, deaths = {self.death}"

#TODO
#to get thing, just to Factions(name).name?

#CONSTANTS
STRUCTURE_KILL = "\"structure_kill\""
KILLED = "killed"
JOINED_TEAM = "joined team"
CHAT = "say"
TEAM_CHAT = "say_team"
ROUND_START = "World triggered \"Round_Start\""


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
        if value[25:].strip() == ROUND_START:
            return all_lines[len(all_lines) - i - 1:]

def get_match_info(match_details):
    #TODO add method for getting the type
    return Modes.CENTAURI_VS_SOL, [Factions.CENTAURI, Factions.SOL]

def get_commanders(match_log_info, match_mode_info, all_players):
    #TODO complete the implementation once logging bug is fixed
    #get the person with the max time as commander
    #make it so that it can deal with duplicate names.

    # commander_joined_pattern = r'"(.*?)<(.*?)><(.*?)><(.*?)>" triggered "took_command" \(structure "(.*)"\) \(struct_team "(.*)"\)'
    commander_joined_pattern = r'Promoted <color=#7070FF>(.*?)<color=#DDE98C> to commander for <color=#7070FF>(.*?)"'
    commander_left_pattern = r'left command'
    #mode, factions = match_mode_info incase i add more checks
    all_commanders = collections.defaultdict(list)
    commander_durations = collections.defaultdict(list)
    commander_log_info = filter(lambda x: "command" in x, match_log_info)
    #TODO, fix to set, else leave it as is.
    for i in commander_log_info:
        joined_match = re.search(commander_joined_pattern, i)
        left_match = re.search(commander_left_pattern, i)
        if joined_match:
            date_string = i[1:23].strip()
            start_time = datetime.strptime(date_string, "%m-%d-%Y - %H:%M:%S")
            commander = joined_match.group(1)
            faction_type = joined_match.group(2)
            #TODOm change to id
            all_commanders[Factions(faction_type).name].append(all_players[commander])
            commander_durations[commander].append(start_time)
        elif left_match:
            date_string = i[1:23].strip()
            end_time = datetime.strptime(date_string, "%m-%d-%Y - %H:%M:%S")
            commander = left_match.group(1)
            faction_type = left_match.group(2)
            commander_durations[commander].append(end_time)
    end_time = ""
    for duration in commander_durations.values():
        if len(duration) % 2 == 0:
            duration.append(end_time)
    all_factions_match = list(commander_durations.keys())
    final_commander = {}
    for faction, commanders in all_factions_match:
        max_duration = -1
        faction_commander = None
        for commander in commanders:
            commander_duration = commander_durations[commander]
            commander_duration = sum([commander_duration[i + 1] - commander_duration[i]
                       for i in range(0, len(commander_duration), 2)])
            if commander_duration > max_duration:
                max_duration = commander_duration
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
            player_id = int(match.group(3))
            #TODO add victim things
            player_faction = match.group(4)
            enemy_name = match.group(5)
            enemy_id = int(match.group(7))
            enemy_faction = match.group(8)
            if enemy_id != "":
                all_players[(enemy_id, Factions(enemy_faction))].update_death()
            all_players[(player_id, Factions(player_faction))].update_unit_kill(enemy_name)#change/fix this

def probability(rating1, rating2):
    return 1.0 * 1.0 / (1 + 1.0 * math.pow(10, 1.0 * (rating1 - rating2) / 400))

def elo_rating_fps(Ra, Rb, K, d):
    #TODO
    #calculate based on points. use maybe an exponential thing?
    pass

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
def checking():
    file_pointer = open("./test_file", "r")
    all_lines = file_pointer.readlines()
    team = get_winning_team(all_lines)
    # print(team)
    # all_players_in_game = {}
    # match_log_info = get_current_match(all_lines)
    # match_mode_info = get_match_info(match_log_info)
    # all_essential_info = list(filter(remove_chat_messages, all_lines))
    # all_players = get_all_players(all_essential_info)
    # # get_commanders(all_lines, match_mode_info, all_players)
    # all_essential_info = [remove_date_data(line) for line in all_essential_info]
    # process_structure_kills(all_essential_info, all_players)
    # process_unit_kills(all_essential_info, all_players)
    # for i in all_players.values():
        # print(i)
        # # input()
    # file_pointer.close()
# cProfile.run('checking()')
