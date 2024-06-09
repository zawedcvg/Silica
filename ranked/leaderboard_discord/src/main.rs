use rust_xlsxwriter::*;
//use futures::TryStreamExt;
use tokio_stream::StreamExt;

use tokio::task::JoinSet;

use dotenv::dotenv;
use sqlx::postgres::{PgPoolOptions, Postgres};
use sqlx::prelude::FromRow;
use sqlx::Pool;
use std::env;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, FromRow)]
struct CommanderEloRecord {
    username: String,
    faction: String,
    ELO: i32,
}

#[derive(Serialize, Deserialize, FromRow)]
struct FpsRankingTotalRecord {
    username: String,
    faction: String,
    total: i64,
    num_matches: i64,
    avg: i64,
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
struct FpsRankingAverageRecord {
    username: String,
    faction: String,
    avg: i64,
    num_matches: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    // Get the database URL from the environment variable
    let database_url = env::var("DATABASE_URL").unwrap();
    println!("{}", database_url);
    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let query_commander = r#"
        SELECT p.username, f.name as faction, rc."ELO"
        FROM players p, factions f, rankings_commander rc
        WHERE rc.player_id = p.id
        AND f.id = rc.faction_id
        ORDER BY rc."ELO" DESC;
        "#;

    let query_fps_total = r#"
        SELECT p.username, f.name as faction, SUM(rc.total_points) as total, COUNT(rc.total_points) as num_matches, SUM(rc.total_points)/COUNT(rc.total_points) as avg
        FROM players p, factions f, matches_players_fps rc
        WHERE rc.player_id = p.id AND f.id = rc.faction_id
        GROUP BY (f.name, p.username)
        ORDER BY SUM(rc.total_points) DESC;
        "#;

    let query_fps_average = r#"
        SELECT p.username, f.name as faction, SUM(rc.total_points)/COUNT(rc.total_points) as avg, COUNT(rc.total_points) as num_matches
        FROM players p, factions f, matches_players_fps rc 
        WHERE rc.player_id = p.id AND f.id = rc.faction_id AND rc.total_points <> 0 
        GROUP BY (f.name, p.username) 
        HAVING COUNT(rc.total_points) > 1
        ORDER BY SUM(rc.total_points)/COUNT(rc.total_points) DESC;
        "#;

    let mut all_tasks = JoinSet::new();

    all_tasks.spawn(create_workbook::<CommanderEloRecord>(
        pool.clone(),
        query_commander,
        String::from("commander_elo_test.xlsx"),
    ));
    all_tasks.spawn(create_workbook::<FpsRankingTotalRecord>(
        pool.clone(),
        query_fps_total,
        String::from("ranking_fps_total.xlsx"),
    ));
    all_tasks.spawn(create_workbook::<FpsRankingAverageRecord>(
        pool.clone(),
        query_fps_average,
        String::from("ranking_fps_average.xlsx"),
    ));

    while let Some(res) = all_tasks.join_next().await {
        res.unwrap();
    }

    Ok(())
}

async fn create_workbook<T>(pool: Pool<Postgres>, query: &str, file_name: String)
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>
        + Send
        + Unpin
        + serde::Serialize
        + for<'a> serde::Deserialize<'a>,
{
    let rows_future = sqlx::query_as::<_, T>(query)
        .fetch_all(&pool);

    let mut workbook = Workbook::new();

    // Add a worksheet to the workbook.
    let worksheet = workbook.add_worksheet();

    // Add some formats to use with the serialization data.
    let header_format = Format::new()
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_background_color("C6E0B4");

    worksheet
        .deserialize_headers_with_format::<T>(0, 0, &header_format)
        .unwrap();


    let rows = rows_future.await.unwrap();
    worksheet.serialize(&rows).unwrap();

    workbook.save(&file_name).unwrap();
    println!("Done with {}", file_name);
    //return ;
}
