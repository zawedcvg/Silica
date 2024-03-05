import asyncio
import sys
from prisma import Prisma
from parser_ranked import checking

modes_id = {"HUMANS_VS_ALIENS": 0, "HUMANS_VS_HUMANS": 1, "HUMANS_VS_HUMANS_VS_ALIENS": 2}
# factions_id = {"Alien": 0, "Centauri": 1, "Sol": 2, "Wildlife": 3}
factions_id = {"Alien": 0, "Human (Centauri)": 1, "Human (Sol)": 2, "Wildlife": 3}

async def main(match_type_info, winning_team, all_players) -> None:
    prisma = Prisma()
    await prisma.connect()
    # print(prisma)
    mode, _, duration = match_type_info
    winning_team_faction_id = factions_id[winning_team.value]
    mode_id = modes_id[mode.value]

    # await prisma.matches.delete_many()
    # await prisma.matches_players_commander.delete_many()
    # await prisma.matches_players_fps.delete_many()
    # exit()

    thing = await prisma.matches.create(
        data={
            'match_length': duration,
            'maps_id': 0,
            'modes_id': mode_id,
            'match_won_faction_id': winning_team_faction_id,
        },
    )

    match_id = thing.id
    tasks = []

    player_fps_info = []
    player_commander_info = []


    for player in all_players.items():
        player_info, player_object = player
        steam_id, faction = player_info
        # ta
        tasks.append(asyncio.create_task(prisma.players.find_first(where={'steam_id': steam_id})))

    all_outputs = await asyncio.gather(*tasks)
    print("done waiting")
    for output, player in zip(all_outputs, all_players.items()):
        player_info, player_object = player
        steam_id, _ = player_info
        if output is None:
            insertion = await prisma.players.create(data={
                'username': player_object.player_name,
                'steam_id': steam_id,
                })
            player_id = insertion.id
        else:
            player_id = output.id
        if player_object.is_fps():
            tier_one_unit_kills, tier_two_unit_kills, tier_three_unit_kills = player_object.unit_kill
            tier_one_structures, tier_two_structures, tier_three_structures = player_object.structure_kill
            to_insert = {
                'player_id': player_id,
                'match_id': match_id,
                'faction_id': factions_id[player_object.faction_type.value],
                'tier_one_kills': tier_one_unit_kills,
                'tier_two_kills': tier_two_unit_kills,
                'tier_three_kills': tier_three_unit_kills,
                'tier_one_structures_destroyed': tier_one_structures,
                'tier_two_structures_destroyed': tier_two_structures,
                'tier_three_structures_destroyed': tier_three_structures,
                'deaths': player_object.death,
                'total_points': player_object.points
                }
            player_fps_info.append(to_insert)
        if player_object.is_commander:
            to_insert = {
                'player_id': player_id,
                'match_id': match_id,
                'faction_id': factions_id[player_object.faction_type.value],
            }
            player_commander_info.append(to_insert)
    # print("here")
    output1 = prisma.matches_players_fps.create_many(player_fps_info)
    output2 = prisma.matches_players_commander.create_many(player_commander_info)
    await output1
    await output2


    # await prisma.factions.create(
        # data={
            # 'name': 'Centauri',
            # 'id': 1,
        # },
    # )

    # await prisma.factions.create(
        # data={
            # 'name': 'Sol',
            # 'id': 2,
        # },
    # )

    # await prisma.factions.create(
        # data={
            # 'name': 'Wildlife',
            # 'id': 3,
        # },
    # )

   # # //prisma.factions. 
    # # print(faction)
    # # factions = await prisma.factions.find_many()
    # # print(factions)
    # await prisma.factions.delete_many(where={'name': 'Aliens'})

    await prisma.disconnect()

if __name__ == '__main__':
    match_type_info, winning_team, all_players = checking(sys.argv[1])
    # print(match_type_info)
    mode, _, duration = match_type_info

    print(modes_id[mode.value])
    print(factions_id[winning_team.value])
    for i in all_players.items():
        print(i)
    asyncio.run(main(match_type_info, winning_team, all_players))
