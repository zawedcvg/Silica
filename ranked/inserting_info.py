import asyncio
import sys
from prisma import Prisma
from parser_ranked import checking, checking_all, elo_rating_commander

from dotenv import load_dotenv

#Change this to the correct .env
load_dotenv(r"C:\Program Files (x86)\Steam\steamapps\common\Silica\UserData\.env")

MODES_ID = {"HUMANS_VS_ALIENS": 0, "HUMANS_VS_HUMANS": 1, "HUMANS_VS_HUMANS_VS_ALIENS": 2}
FACTIONS_ID = {"Alien": 0, "Centauri": 1, "Sol": 2, "Wildlife": 3}
MAP_ID = {}

def get_fps_data(player, player_id, match_id):
    tier_one_unit_kills, tier_two_unit_kills, tier_three_unit_kills = player.unit_kill
    tier_one_structures, tier_two_structures, tier_three_structures = player.structure_kill
    to_insert = {
        'player_id': player_id,
        'match_id': match_id,
        'faction_id': FACTIONS_ID[player.faction_type.value],
        'tier_one_kills': tier_one_unit_kills,
        'tier_two_kills': tier_two_unit_kills,
        'tier_three_kills': tier_three_unit_kills,
        'tier_one_structures_destroyed': tier_one_structures,
        'tier_two_structures_destroyed': tier_two_structures,
        'tier_three_structures_destroyed': tier_three_structures,
        'deaths': player.death,
        'total_points': player.points
        }
    return to_insert

async def main(match_type_info, winning_team, all_players, current_map) -> None:
    prisma = Prisma()
    await prisma.connect()

    mode, _, duration = match_type_info
    winning_team_faction_id = FACTIONS_ID[winning_team.value]
    mode_id = MODES_ID[mode.value]

    #dont remove, this is for clearing the database
    # await prisma.matches.delete_many()
    # await prisma.players.delete_many()
    # await prisma.matches_players_commander.delete_many()
    # await prisma.matches_players_fps.delete_many()
    # await prisma.rankings_commander.delete_many()
    # exit()
    if current_map == None:
        maps_id = 0
    else:
        #TODO replace
        maps_id = 0

    match_info = await prisma.matches.create(
        data={
            'match_length': duration,
            'maps_id': maps_id,
            'modes_id': mode_id,
            'match_won_faction_id': winning_team_faction_id,
        },
    )


    match_id = match_info.id
    tasks = []
    commander_info = []

    player_fps_info = []
    player_commander_info = []


    for player in all_players.items():
        player_info, player_object = player
        steam_id, _ = player_info
        # ta
        tasks.append(asyncio.create_task(prisma.players.find_first(where={'steam_id': steam_id})))

    already_added_steam_ids = {}

    all_outputs = await asyncio.gather(*tasks)
    for output, player in zip(all_outputs, all_players.items()):
        player_info, player_object = player
        steam_id, _ = player_info
        if output is None and steam_id not in already_added_steam_ids:
            insertion = await prisma.players.create(data={
                'username': player_object.player_name,
                'steam_id': steam_id,
                })
            player_id = insertion.id
            already_added_steam_ids[steam_id] = player_id
        elif output is None:
            player_id = already_added_steam_ids[steam_id]
        else:
            player_id = output.id

        faction_id = FACTIONS_ID[player_object.faction_type.value]
        if player_object.is_fps():
            to_insert = get_fps_data(player_object, player_id, match_id)
            player_fps_info.append(to_insert)
        if player_object.is_commander:
            to_insert = {
                'player_id': player_id,
                'match_id': match_id,
                'faction_id': faction_id
            }
            commander_info.append(asyncio.create_task(prisma.rankings_commander.find_first(where={'player_id': player_id, 'faction_id': faction_id})))
            player_commander_info.append(to_insert)

    all_commander_future = asyncio.gather(*commander_info)
    output1 = prisma.matches_players_fps.create_many(player_fps_info)
    output2 = prisma.matches_players_commander.create_many(player_commander_info)
    await output1
    await output2
    all_commanders_result = await all_commander_future

    elos = []

    for output, player in zip(all_commanders_result, player_commander_info):
        if output is None:
            insertion = await prisma.rankings_commander.create(data={
                'player_id': player['player_id'],
                'faction_id': player['faction_id'],
                'ELO': 1000,
                'wins': 0
                })
            elos.append(1000)
        else:
            elos.append(output.ELO)

    all_faction_ids = [player['faction_id'] for player in player_commander_info]
    win_list = [faction_id == winning_team_faction_id for faction_id in all_faction_ids]
    new_elos = elo_rating_commander(elos, win_list)

    elos_commander_updates = []
    for player, new_elo in zip(player_commander_info, new_elos):
        update = prisma.rankings_commander.update_many(where={
            'player_id': player['player_id'],
            'faction_id': player['faction_id']}, data={'ELO': new_elo})
        elos_commander_updates.append(update)
    await asyncio.gather(*elos_commander_updates)

    await prisma.disconnect()

if __name__ == '__main__':
    sys.stdout = open('parser_stdout.txt', 'a')
    sys.stderr = open('parser_stderr.txt', 'a')
    print("Starting the process")
    print("Parsing the info")
    all_parse_info = checking(sys.argv[1])
    print("Parsing succesful")
    all_parse_info = [all_parse_info]
    for match_type_info, winning_team, all_players in all_parse_info:
        mode, _, duration = match_type_info
        asyncio.run(main(match_type_info, winning_team, all_players, None))
    sys.stdout.close()
    sys.stderr.close()
