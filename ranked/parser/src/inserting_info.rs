use crate::parser::{Factions, Game, Maps, Modes, Player, TIER_ONE, TIER_THREE, TIER_TWO};
use futures::future::join_all;
//make stuff into a transaction since plan is to add multiple servers.
use futures::FutureExt;
use sqlx::postgres::Postgres;
use sqlx::Pool;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinHandle;

const PLAYER_NOT_FOUND: i32 = -1;

macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.iter().zip(
            zip!($($y), +))
    )
}

pub async fn inserting_info(
    game: Arc<Game>,
    pool: Pool<Postgres>,
) -> Result<(), Box<dyn std::error::Error>> {
    //let tasks = Vec::new();
    println!("Connection done");

    //let mut all_tasks = JoinSet::new();
    let bulk_search_future: Pin<Box<JoinHandle<Box<dyn Any + Send>>>> = Box::pin(tokio::spawn({
        let new_thing = game.clone();
        let pool_clone = pool.clone();
        async move {
            let result = bulk_search_player_ids(new_thing.get_player_vec(), pool_clone).await;
            Box::new(result) as Box<dyn Any + Send>
        }
    }));

    let match_id_future: Pin<Box<JoinHandle<Box<dyn Any + Send>>>> = Box::pin(tokio::spawn({
        let pool_clone = pool.clone();
        let game_clone = game.clone();
        async move {
            let result = insert_into_match(&game_clone, pool_clone).await;
            Box::new(result) as Box<dyn Any + Send>
        }
    }));

    //tasks.push(bulk_search_future);
    //tasks.push(match_id_future);

    let mut already_added_steam_ids: HashMap<i64, i32> = HashMap::new();

    let mut fps_bulk_insert: Vec<_> = Vec::new();
    let mut commander_bulk_insert: Vec<_> = Vec::new();
    let mut commander_details_futures: Vec<_> = Vec::new();

    let now = Instant::now();
    let futures_vec = vec![bulk_search_future, match_id_future];

    let results = join_all(futures_vec).await;
    let bulk_search_players = match &results[0] {
        Ok(thing) => thing.downcast_ref::<Vec<i32>>().unwrap(),
        Err(_) => panic!("Spmething went wrong"),
    };
    let match_id = match &results[1] {
        Ok(thing) => thing.downcast_ref::<i32>().unwrap(),
        Err(_) => panic!("Spmething went wrong"),
    };
    println!("{:?}", now.elapsed());

    for (&player, &query_output) in game.get_player_vec().iter().zip(bulk_search_players) {
        let db_player_id: i32;
        if query_output != PLAYER_NOT_FOUND {
            db_player_id = query_output
        } else {
            match already_added_steam_ids.get(&player.player_id) {
                Some(&id) => db_player_id = id,
                None => {
                    db_player_id = insert_into_players(player, pool.clone()).await;
                    already_added_steam_ids.insert(player.player_id, db_player_id);
                }
            }
        }

        if player.is_fps() {
            fps_bulk_insert.push((db_player_id, match_id.to_owned(), player));
        }

        if player.is_commander {
            commander_bulk_insert.push((db_player_id, match_id.to_owned(), player));
            commander_details_futures.push(search_for_commander(
                db_player_id,
                player,
                pool.clone(),
            ));
        }
    }

    let mut insert_new_commander: Vec<_> = Vec::new();

    let mut win_list: Vec<_> = Vec::new();

    let bulk_fps_insert_future =
        bulk_insert_into_matches_players_fps(fps_bulk_insert, pool.clone()).boxed();

    //can change this to do a process as we get the details. no need to wait.
    let returned_elos = join_all(commander_details_futures).await;
    println!("Got the returned elos now");

    let mut elo_list = Vec::new();
    for ((db_player_id, _, player), returned_elo) in zip!(commander_bulk_insert, returned_elos) {
        match returned_elo {
            Some(elo) => {
                elo_list.push(elo);
            }
            None => {
                insert_new_commander.push(
                    insert_into_rankings_commander(db_player_id.to_owned(), player, pool.clone())
                        .boxed(),
                );
                elo_list.push(1000);
            }
        }
        win_list.push(player.did_win(game.winning_team));
    }
    let new_elos = elo_rating_commander(elo_list, &win_list, 30);

    println!("{:?}", new_elos);

    let bulk_commander_insert_future =
        bulk_insert_into_matches_players_commander(&commander_bulk_insert, pool.clone()).boxed();

    let a =
        update_commander_elo(&new_elos, &commander_bulk_insert, &win_list, pool.clone()).boxed();

    println!("Waiting for commander elo update");

    let mut tasks = vec![bulk_fps_insert_future, a, bulk_commander_insert_future];
    tasks.extend(insert_new_commander);
    join_all(tasks).await;
    //join_all(insert_new_commander).await;

    Ok(())
}

fn probability(rating1: i32, rating2: i32) -> f64 {
    let base: f64 = 10.0;
    1.0 * 1.0 / (1.0 + 1.0 * base.powf(1.0 * (f64::from(rating1 - rating2) / 400.0)))
}

fn elo_rating_commander(elo_list: Vec<i32>, win_list: &[bool], k: i32) -> Vec<i32> {
    //refactor this entire thing. lot of duplicated code
    let mut new_elo: Vec<i32> = Vec::new();
    if elo_list.is_empty() {
        new_elo
    } else if elo_list.len() < 3 {
        let r_a = elo_list[0];

        let r_b = if elo_list.len() == 1 {
            1000
        } else {
            elo_list[1]
        };
        let p_b_win = probability(r_a, r_b);
        let p_a_win = probability(r_b, r_a);
        let new_elo_ra: f64;
        let new_elo_rb: f64;

        if win_list[0] {
            new_elo_ra = f64::from(r_a) + f64::from(k) * (1.0 - p_a_win);
            new_elo_rb = f64::from(r_b) + f64::from(k) * (0.0 - p_b_win);
        } else {
            new_elo_ra = f64::from(r_a) + f64::from(k) * (0.0 - p_a_win);
            new_elo_rb = f64::from(r_b) + f64::from(k) * (1.0 - p_b_win);
        }

        new_elo.push(new_elo_ra as i32);
        if elo_list.len() == 2 {
            new_elo.push(new_elo_rb as i32);
        }
        new_elo
    } else if elo_list.len() == 3 {
        let r_a = elo_list[0];
        let r_b = elo_list[1];
        let r_c = elo_list[2];
        let mut probability_list: [f64; 3] = [0.0, 0.0, 0.0];
        probability_list[0] = probability(r_a, r_b) + probability(r_a, r_c);
        probability_list[1] = probability(r_b, r_a) + probability(r_b, r_c);
        probability_list[2] = probability(r_c, r_a) + probability(r_c, r_b);
        let mut new_elo: Vec<i32> = Vec::new();

        for (p, (&w, r)) in zip!(probability_list, win_list, elo_list) {
            let did_win: f64 = if w { 1.0 } else { 0.0 };
            let new_elo_thing = f64::from(r) + f64::from(k) * 2.0 * (did_win - p / 6.0);
            new_elo.push(new_elo_thing as i32);
        }
        new_elo
    } else {
        panic!("More than expected commanders")
    }
}

async fn insert_into_players(player: &Player, pool: Pool<Postgres>) -> i32 {
    let id_future = sqlx::query!(
        r#"
        INSERT INTO players ( username, steam_id )
        VALUES ( $1, $2 )
        RETURNING id
        "#,
        player.player_name,
        player.player_id,
    )
    .fetch_one(&pool)
    .await;

    let id = match id_future {
        Ok(id) => id,
        Err(e) => panic!("Could not add player {} due to {e}", {
            player.player_name.to_string()
        }),
    };
    id.id
}

async fn bulk_insert_into_matches_players_fps(
    to_insert_thing: Vec<(i32, i32, &Player)>,
    pool: Pool<Postgres>,
) {
    let id_future = sqlx::query(
        r#"
        INSERT INTO matches_players_fps ( player_id, match_id, faction_id, tier_one_kills, tier_two_kills, tier_three_kills, tier_one_structures_destroyed, tier_two_structures_destroyed, tier_three_structures_destroyed, total_points, deaths )
        SELECT * FROM UNNEST( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11 )
        "#)
        .bind(to_insert_thing.iter().map(|a| a.0).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| a.1).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| get_faction_id(&a.2.faction_type)).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| a.2.unit_kill[TIER_ONE]).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| a.2.unit_kill[TIER_TWO]).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| a.2.unit_kill[TIER_THREE]).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| a.2.structure_kill[TIER_ONE]).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| a.2.structure_kill[TIER_TWO]).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| a.2.structure_kill[TIER_THREE]).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| a.2.points).collect::<Vec<i32>>())
        .bind(to_insert_thing.iter().map(|a| a.2.death).collect::<Vec<i32>>())
    .execute(&pool)
    .await;

    match id_future {
        Ok(_) => (),
        Err(e) => panic!("Could not add player due to {e}"),
    };
}

async fn _insert_into_matches_players_fps(
    db_player_id: i32,
    db_match_id: i32,
    player: &Player,
    pool: Pool<Postgres>,
) {
    let faction_id = HashMap::from([
        (Factions::Alien, 0),
        (Factions::Centauri, 1),
        (Factions::Sol, 2),
        (Factions::Wildlife, 3),
    ]);
    let id_future = sqlx::query!(
        r#"
        INSERT INTO matches_players_fps ( player_id, match_id, faction_id, tier_one_kills, tier_two_kills, tier_three_kills, tier_one_structures_destroyed, tier_two_structures_destroyed, tier_three_structures_destroyed, total_points, deaths )
        VALUES ( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11 )
        "#,
        db_player_id,
        db_match_id,
        faction_id.get(&player.faction_type),
        player.unit_kill[TIER_ONE],
        player.unit_kill[TIER_TWO],
        player.unit_kill[TIER_THREE],
        player.structure_kill[TIER_ONE],
        player.structure_kill[TIER_TWO],
        player.structure_kill[TIER_THREE],
        player.points,
        player.death
    )
    .execute(&pool)
    .await;

    match id_future {
        Ok(_) => (),
        Err(e) => panic!("Could not add player {} due to {e}", {
            player.player_name.to_string()
        }),
    };
}

async fn _search_for_player(player: &Player, pool: Pool<Postgres>) -> Option<i32> {
    let id_future = sqlx::query!(
        r#"
        SELECT id FROM players
        WHERE steam_id=$1
        "#,
        player.player_id,
    )
    .fetch_one(&pool)
    .await;

    match id_future {
        Ok(id) => Some(id.id),
        Err(_) => None,
    }
}

async fn search_for_commander(
    db_player_id: i32,
    player: &Player,
    pool: Pool<Postgres>,
) -> Option<i32> {
    println!("Starting the search for commander 1");
    let id_future = sqlx::query!(
        r#"
        SELECT * FROM rankings_commander
        WHERE player_id=$1
        AND faction_id=$2
        "#,
        db_player_id,
        get_faction_id(&player.faction_type)
    )
    .fetch_one(&pool)
    .await;

    match id_future {
        Ok(id) => Some(id.ELO),
        Err(_) => None,
    }
}

fn get_faction_id(faction: &Factions) -> i32 {
    let faction_id = HashMap::from([
        (Factions::Alien, 0),
        (Factions::Centauri, 1),
        (Factions::Sol, 2),
        (Factions::Wildlife, 3),
    ]);
    *faction_id
        .get(faction)
        .unwrap_or_else(|| panic!("Could not find the faction id of the faction"))
}

async fn insert_into_match(game: &Game, pool: Pool<Postgres>) -> i32 {
    println!("Something started for insert into match");
    let maps_id = HashMap::from([
        (Maps::NarakaCity, 1),
        (Maps::MonumentValley, 2),
        (Maps::RiftBasin, 3),
        (Maps::Badlands, 4),
        (Maps::GreatErg, 5),
    ]);
    let modes_id = HashMap::from([
        (Modes::SolVsAlien, 0),
        (Modes::CentauriVsSol, 1),
        (Modes::CentauriVsSolVsAlien, 2),
    ]);

    let id_future = sqlx::query!(
        r#"
    INSERT INTO matches ( match_length, modes_id, maps_id, match_won_faction_id )
    VALUES ( $1, $2, $3, $4 )
    RETURNING id
    "#,
        game.get_match_length(),
        modes_id.get(&game.match_type),
        maps_id.get(&game.map),
        get_faction_id(&game.winning_team)
    )
    .fetch_one(&pool)
    .await;

    let id = match id_future {
        Ok(id) => id,
        Err(e) => panic!("Could add match due to {e}"),
    };
    id.id
}

async fn _insert_into_matches_players_commander(
    db_player_id: i32,
    db_match_id: i32,
    player: &Player,
    pool: Pool<Postgres>,
) {
    let faction_id = HashMap::from([
        (Factions::Alien, 0),
        (Factions::Centauri, 1),
        (Factions::Sol, 2),
        (Factions::Wildlife, 3),
    ]);
    let id_future = sqlx::query!(
        r#"
        INSERT INTO matches_players_commander ( player_id, match_id, faction_id )
        VALUES ( $1, $2, $3 )
        "#,
        db_player_id,
        db_match_id,
        faction_id.get(&player.faction_type).unwrap(),
    )
    .execute(&pool)
    .await;

    match id_future {
        Ok(_) => (),
        Err(e) => panic!("Could not add player {} due to {e}", {
            player.player_name.to_string()
        }),
    };
}

async fn insert_into_rankings_commander(db_player_id: i32, player: &Player, pool: Pool<Postgres>) {
    println!("Inserting into rankings commanders");
    let id_future = sqlx::query!(
        r#"
        INSERT INTO rankings_commander ( player_id, faction_id, wins, "ELO" )
        VALUES ( $1, $2, $3, $4 )
        "#,
        db_player_id,
        get_faction_id(&player.faction_type),
        0,
        1000,
    )
    .execute(&pool)
    .await;

    match id_future {
        Ok(_) => (),
        Err(e) => panic!(
            "Could not add player {} to rankings_commander due to {e}",
            { player.player_name.to_string() }
        ),
    };
}

async fn bulk_insert_into_matches_players_commander(
    to_insert_thing: &[(i32, i32, &Player)],
    pool: Pool<Postgres>,
) {
    let id_future = sqlx::query(
        r#"
        INSERT INTO matches_players_commander ( player_id, match_id, faction_id )
        SELECT * FROM UNNEST( $1, $2, $3 )
        "#,
    )
    .bind(to_insert_thing.iter().map(|a| a.0).collect::<Vec<i32>>())
    .bind(to_insert_thing.iter().map(|a| a.1).collect::<Vec<i32>>())
    .bind(
        to_insert_thing
            .iter()
            .map(|a| get_faction_id(&a.2.faction_type))
            .collect::<Vec<i32>>(),
    )
    .execute(&pool)
    .await;

    match id_future {
        Ok(_) => (),
        Err(e) => panic!("Could not add commanders due to {e}"),
    };
}

async fn update_commander_elo(
    new_elos: &[i32],
    to_insert_thing: &[(i32, i32, &Player)],
    win_list: &[bool],
    pool: Pool<Postgres>,
) {
    let id_future = sqlx::query(
        r#"
        UPDATE rankings_commander
        SET "ELO" = u.new_elo, wins=wins+u.is_win
        FROM (
            SELECT unnest($1::integer[]) AS new_elo,
            unnest($2::integer[]) AS pid,
            unnest($3::integer[]) AS fid,
            unnest($4::integer[]) AS is_win
            ) AS u
        WHERE player_id = u.pid AND faction_id = u.fid
        "#,
    )
    .bind(new_elos)
    .bind(to_insert_thing.iter().map(|a| a.0).collect::<Vec<i32>>())
    .bind(
        to_insert_thing
            .iter()
            .map(|a| get_faction_id(&a.2.faction_type))
            .collect::<Vec<i32>>(),
    )
    .bind(
        win_list
            .iter()
            .map(|&x| if x { 1 } else { 0 })
            .collect::<Vec<i32>>(),
    )
    .execute(&pool)
    .await;

    match id_future {
        Ok(_) => (),
        Err(e) => panic!("Could not add commanders due to {e}"),
    };
}

async fn bulk_search_player_ids(players: Vec<&Player>, pool: Pool<Postgres>) -> Vec<i32> {
    println!("Something started for bulk player search");
    let all_search_ids = sqlx::query!(
        r#"
        SELECT COALESCE(u.id, -1) AS id
        FROM UNNEST($1::BigInt[]) WITH ORDINALITY as p(id, ord)
        LEFT JOIN players u ON u.steam_id = p.id
        ORDER BY p.ord
        "#,
        &players
            .iter()
            .map(|player| player.player_id)
            .collect::<Vec<i64>>()
    )
    .fetch_all(&pool)
    .await;

    match all_search_ids {
        Ok(all_searched) => all_searched
            .iter()
            .map(|x| x.id.unwrap_or_else(|| panic!("Could not unwrap id")))
            .collect::<Vec<_>>(),
        Err(e) => panic!("could not update due to {e}"),
    }
}

//async fn bulk_search_commander_ids(players: Vec<&Player>, pool: Pool<Postgres>) -> Vec<i32> {
//let all_search_ids = sqlx::query!(
//r#"
//SELECT COALESCE(u.id, -1) AS id
//FROM UNNEST($1::BigInt[]) WITH ORDINALITY as p(id, ord)
//LEFT JOIN  u ON u.steam_id = p.id
//ORDER BY p.ord
//"#,
//&players
//.iter()
//.map(|player| player.player_id)
//.collect::<Vec<i64>>()
//)
//.fetch_all(&pool)
//.await;

//match all_search_ids {
//Ok(all_searched) => all_searched
//.iter()
//.map(|x| x.id.unwrap_or_else(|| panic!("Could not unwrap id")))
//.collect::<Vec<_>>(),
//Err(e) => panic!("could not update due to {e}"),
//}
//}
