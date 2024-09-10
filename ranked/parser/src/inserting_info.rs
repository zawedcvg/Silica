use crate::parser::{Factions, Game, Maps, Modes, Player, TIER_ONE, TIER_THREE, TIER_TWO};
use futures::future::join_all;
use skillratings::{
    weng_lin::{weng_lin_multi_team, WengLinConfig, WengLinRating},
    MultiTeamOutcome,
};
//make stuff into a transaction since plan is to add multiple servers.
use futures::FutureExt;
use log::{debug, info, warn};
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
    info!("Connected to the database");

    // NOTE This is temporary and only until the game AI is better

    if game.get_player_vec().len() < 7 {
        warn!("Not enough players. Exiting the program.");
        return Ok(());
    }

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

    let mut already_added_steam_ids: HashMap<i64, i32> = HashMap::new();

    let mut fps_bulk_insert: Vec<_> = Vec::new();
    let mut commander_bulk_insert: Vec<_> = Vec::new();
    let mut commander_details_futures: Vec<_> = Vec::new();

    let now = Instant::now();
    let futures_vec = vec![bulk_search_future, match_id_future];

    let results = join_all(futures_vec).await;
    let bulk_search_players = match &results[0] {
        Ok(thing) => thing.downcast_ref::<Vec<i32>>().unwrap(),
        Err(_) => panic!("Something went wrong"),
    };
    let match_id = match &results[1] {
        Ok(thing) => thing.downcast_ref::<i32>().unwrap(),
        Err(_) => panic!("Something went wrong"),
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
                    //Can mmake this better by using bulk
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

    let returned_elos = bulk_search_commander_ids(&commander_bulk_insert, pool.clone()).await;

    println!("Got the returned elos now");

    let mut elo_list = Vec::new();
    for ((db_player_id, _, player), (returned_rating, returned_uncertainty)) in zip!(commander_bulk_insert, returned_elos) {
        if returned_rating != -1.0 {
            elo_list.push((returned_rating, returned_uncertainty));
        } else {
            insert_new_commander.push(
                insert_into_rankings_commander(db_player_id.to_owned(), player, pool.clone())
                    .boxed(),
            );
            elo_list.push((25.0, 8.3333));
        }
        if player.did_win(game.winning_team) {
            win_list.push(1);
        } else {
            win_list.push(2);
        }
    }
    let new_elos = elo_rating_commander(elo_list, &win_list);

    println!("{:?}", new_elos);

    let bulk_commander_insert_future =
        bulk_insert_into_matches_players_commander(&commander_bulk_insert, pool.clone()).boxed();

    let a =
        update_commander_elo(&new_elos[..], &commander_bulk_insert, &win_list, pool.clone()).boxed();

    println!("Waiting for commander elo update");

    let mut tasks = vec![bulk_fps_insert_future, a, bulk_commander_insert_future];
    tasks.extend(insert_new_commander);
    join_all(tasks).await;
    //join_all(insert_new_commander).await;

    Ok(())
}

fn elo_rating_commander(elo_list: Vec<(f64, f64)>, win_list: &[usize]) -> Vec<Vec<WengLinRating>> {
    let mut team_list: Vec<_> = Vec::new();

    for (index, &(rating, uncertainty)) in elo_list.iter().enumerate() {
        let rating_vec = vec![WengLinRating {
            rating,
            uncertainty,
        }]; // Create the vector here
        team_list.push((rating_vec, MultiTeamOutcome::new(win_list[index])));
    }

    //let new_teams = weng_lin_multi_team(&team_list[..], &WengLinConfig::new());
    let new_teams = weng_lin_multi_team(
        &team_list
            .iter()
            .map(|(ratings, outcome)| (&ratings[..], *outcome))
            .collect::<Vec<_>>()[..],
        &WengLinConfig::new(),
    );
    new_teams

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
) -> Option<(f64, f64)> {
    debug!("Starting the search for commander {}", player.player_name);
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
        Ok(id) => Some((id.rating, id.uncertainty)),
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
    debug!("Something started for insert into match");
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
    debug!("Inserting into rankings commanders");
    let id_future = sqlx::query!(
        r#"
        INSERT INTO rankings_commander ( player_id, faction_id )
        VALUES ( $1, $2 )
        "#,
        db_player_id,
        get_faction_id(&player.faction_type),
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
    new_ratings: &[Vec<WengLinRating>],
    to_insert_thing: &[(i32, i32, &Player)],
    win_list: &[usize],
    pool: Pool<Postgres>,
) {
    let id_future = sqlx::query(
        r#"
        UPDATE rankings_commander
        SET rating = u.new_rating,
            wins = wins + u.is_win,
            uncertainty = u.new_uncertainty
        FROM (
            SELECT
                UNNEST($1::float[]) AS new_rating,
                UNNEST($2::float[]) AS new_uncertainty,
                UNNEST($3::integer[]) AS pid,
                UNNEST($4::integer[]) AS fid,
                UNNEST($5::integer[]) AS is_win
        ) AS u
        WHERE player_id = u.pid AND faction_id = u.fid
        "#,
    )
    .bind(new_ratings.iter().map(|a| a[0].rating).collect::<Vec<f64>>())
    .bind(new_ratings.iter().map(|a| a[0].uncertainty).collect::<Vec<f64>>())
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
            .map(|&x| if x == 1 { 1 } else { 0 })
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
    debug!("Something started for bulk player search");
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

async fn bulk_search_commander_ids(
    players: &[(i32, i32, &Player)],
    pool: Pool<Postgres>,
) -> Vec<(f64, f64)> {
    let all_search_ids = sqlx::query!(
        r#"
        SELECT COALESCE(r.rating, -1) as rating, COALESCE(r.uncertainty, -1) as uncertainty
        FROM (
            SELECT
                id,
                faction,
                ord
            FROM
                UNNEST($1::INT[], $2::INT[]) WITH ORDINALITY AS t(id, faction, ord)
        ) AS u
        LEFT JOIN rankings_commander r ON r.player_id = u.id AND r.faction_id = u.faction
        ORDER BY u.ord;
        "#,
        &players.iter().map(|x| x.0).collect::<Vec<i32>>(),
        &players
            .iter()
            .map(|a| get_faction_id(&a.2.faction_type))
            .collect::<Vec<i32>>()
    )
    .fetch_all(&pool)
    .await;

    match all_search_ids {
        Ok(all_searched) => all_searched
            .iter()
            .map(|x| {
                (
                    x.rating
                        .unwrap_or_else(|| panic!("Could not unwrap rating")),
                    x.uncertainty
                        .unwrap_or_else(|| panic!("Could not unwrap uncertainty")),
                )
            })
            .collect::<Vec<_>>(),
        Err(e) => panic!("could not update due to {e}"),
    }
}
