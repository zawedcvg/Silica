pub mod inserting_info;
use dotenv::dotenv;
use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};
use log::{debug, error, info};
use sqlx::postgres::PgPoolOptions;
use std::path::Path;
use std::sync::Arc;
pub mod parser;
use crate::inserting_info::inserting_info;
use crate::parser::{checking_folder, parse_all_matches};
use std::env;

#[tokio::main]
async fn main() {
    //Open files to append stdout and stderr
    let args: Vec<String> = env::args().collect();

    let env_path = &args[1];

    match env::set_current_dir(env_path) {
        Ok(_) => (),
        Err(e) => {
            error!(
                "Could not set current directory to {:?} due to {e}.",
                env_path
            );
            panic!()
        }
    }

    dotenv().ok();

    let separator =
        "-------------------------------------------------------------------------------";

    let _logger = Logger::try_with_str("info")
        .unwrap()
        .use_utc()
        .log_to_file(
            FileSpec::default()
                .directory("parser-logs")
                .suppress_timestamp()
                .suffix("log"),
        )
        //.duplicate_to_stdout(flexi_logger::Duplicate::All)
        .append()
        .write_mode(WriteMode::BufferAndFlush)
        .format(|w, now, record| {
            write!(
                w,
                "{} [{}] {}",
                now.format("%Y-%m-%d %H:%M:%S"), // Date and time in the timestamp
                record.level(),
                &record.args()
            )
        })
        .rotate(
            Criterion::Size(5_000_000),
            Naming::TimestampsCustomFormat {
                current_infix: None,
                format: "r%Y-%m-%d",
            },
            Cleanup::KeepLogAndCompressedFiles(5, 10),
        )
        .start()
        .unwrap();

    // Get the database URL from the environment variable
    let database_url = env::var("DATABASE_URL").unwrap();

    // Create a connection pool
    // WARN the .env path is wrong? check
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url);

    let log_folder = Path::new(&args[1]);

    //TODO getting the current map method takes a lot of time due to next line of rev lines/or is

    // info!("parsing the folder");

    let game = checking_folder(log_folder);

    //
    // For debugging
    // let game = parse_all_matches(vec![Path::new(
    //     "/home/neeladri/Silica/ranked/parser/log_folder/L20251215.log",
    // )
    // .to_path_buf()]);
    // // parse_info(vec![path.to_path_buf()])
    //
    // info!("Found {} matches", game.len());
    // for (i, g) in game.iter().enumerate() {
    //     info!(
    //         "Match {}: {:?} on {:?}, winner: {:?}, duration: {}s, players: {}",
    //         i + 1,
    //         g.match_type,
    //         g.map,
    //         g.winning_team,
    //         g.get_match_length(),
    //         g.get_player_vec().len()
    //     );
    // }

    debug!("Match length is {:#?} seconds", game.get_match_length());

    let _ = inserting_info(
        Arc::new(game),
        pool.await.unwrap_or_else(|e| {
            panic!("something went wrong while connecting to database due to {e}")
        }),
    )
    .await;
    info!("{separator}");
}
