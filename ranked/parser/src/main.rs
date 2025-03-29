use dotenv::dotenv;
use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};
use log::{debug, error, info};
use sqlx::postgres::PgPoolOptions;
use std::path::Path;
use std::sync::Arc;
// pub mod inserting_info;
// use crate::inserting_info::inserting_info;
use parser::checking_folder;
use std::env;

#[tokio::main]
async fn main() {
    // Open files to append stdout and stderr
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
    // let pool = PgPoolOptions::new()
    //     .max_connections(10)
    //     .connect(&database_url);

    let log_folder = Path::new(&args[1]);

    //TODO getting the current map method takes a lot of time due to next line of rev lines/or is

    info!("parsing the folder");

    let game = checking_folder(log_folder);

    debug!("Match length is {:#?} seconds", game.get_match_length());

    // let _ = inserting_info(
    //     Arc::new(game),
    //     pool.await.unwrap_or_else(|e| {
    //         panic!("something went wrong while connecting to database due to {e}")
    //     }),
    // )
    // .await;
    info!("{separator}");
}


#[cfg(test)]
mod tests {
    use parser::Game;
    use parser::{checking_file, checking_folder, Factions, Maps, Modes};
    use chrono::NaiveDateTime;
    use std::path::Path;

    #[test]
    fn human_vs_human_single_file() {
        let game = checking_file(Path::new("./test_stuff/some.log"));
        println!("{:#?}", game.players);
        assert_eq!(game.match_type, Modes::CentauriVsSol);
        assert_eq!(game.winning_team, Factions::Centauri);
        assert_eq!(game.get_player_vec().len(), 20);
        assert_eq!(
            game.start_time,
            NaiveDateTime::parse_from_str("07/23/2024 - 01:53:40", "%m/%d/%Y - %H:%M:%S").unwrap()
        );
        assert_eq!(
            game.end_time,
            NaiveDateTime::parse_from_str("07/23/2024 - 02:51:28", "%m/%d/%Y - %H:%M:%S").unwrap()
        );
    }

    #[test]
    fn multiple_file_in_directory() {
        let game = checking_folder(Path::new("./log_folder"));
        assert_eq!(game.match_type, Modes::SolVsAlien);
        assert_eq!(game.winning_team, Factions::Alien);
        assert_eq!(game.map, Maps::MonumentValley);
        assert_eq!(
            game.start_time,
            NaiveDateTime::parse_from_str("07/23/2024 - 13:31:37", "%m/%d/%Y - %H:%M:%S").unwrap()
        );
        assert_eq!(
            game.end_time,
            NaiveDateTime::parse_from_str("07/23/2024 - 14:19:26", "%m/%d/%Y - %H:%M:%S").unwrap()
        );
    }

    #[test]
    fn check_match_type_parsing() {
        let mut game = Game {
        current_match: vec![
            r#"   L 07/22/2024 - 00:18:48: World triggered "Round_Start" (gametype "HUMANS_VS_ALIENS")    "#
                .to_string()], ..Default::default()
        };
        game.get_match_type();
        assert_eq!(game.match_type, Modes::SolVsAlien);

        game.current_match = vec![
            r#" L 07/22/2024 - 12:18:45: World triggered "Round_Start" (gametype "HUMANS_VS_HUMANS_VS_ALIENS")  "#
                .to_string(),
        ];
        game.get_match_type();
        assert_eq!(game.match_type, Modes::CentauriVsSolVsAlien);

        game.current_match = vec![
            r#" L 07/22/2024 - 12:18:45: World triggered "Round_Start" (gametype "HUMANS_VS_HUMANS")  "#
                .to_string(),
        ];
        game.get_match_type();
        assert_eq!(game.match_type, Modes::CentauriVsSol);
    }

    #[test]
    fn check_winning_team_parsing() {
        let mut game = Game {
            current_match: vec![
                r#" L 07/22/2024 - 23:58:42: Team "Alien" triggered "Victory"   "#.to_string(),
            ],
            ..Default::default()
        };
        game.get_winning_team();
        assert_eq!(game.winning_team, Factions::Alien);
        game.current_match =
            vec![r#" L 07/22/2024 - 23:58:42: Team "Centauri" triggered "Victory"   "#.to_string()];
        game.get_winning_team();
        assert_eq!(game.winning_team, Factions::Centauri);

        game.current_match =
            vec![r#" L 07/22/2024 - 23:58:42: Team "Sol" triggered "Victory"   "#.to_string()];
        game.get_winning_team();
        assert_eq!(game.winning_team, Factions::Sol);
    }

    //TODO extensively check commanders parsing

    #[test]
    fn check_abnormal_characters() {
        let mut game = Game {
            current_match: vec![
                r#"L 07/12/2024 - 00:03:57: "ßЉбббппѐѐ<21312><321321312><>" joined team "Alien""#.to_string(),
r#"L 07/12/2024 - 00:09:33: "ßЉбббппѐѐ<21312><321321312><Alien>" triggered "structure_kill" (structure "Node") (struct_team "Alien")"#.to_string(),
r#"L 07/10/2024 - 00:00:57: "ßЉбббппѐѐ<21312><321321312><Alien>" killed "инининин ТУТУХТУХХ<123212><3122211><Sol>" with "Goliath" (dmgtype "Collision") (victim "Soldier_Commando")"#.to_string(),
r#"L 07/10/2024 - 00:00:57: "ßЉбббппѐѐ<21312><321321312><Alien>" killed "инининин ТУХТУХХ<13212><31122211><Alien>" with "Goliath" (dmgtype "Collision") (victim "Goliath")"#.to_string(),
            ],
            ..Default::default()
        };
        game.process_all_players();
        game.process_structure_kills();
        game.process_kills();
        let abnormal_player = game.players.get(&(321321312_i64, Factions::Alien)).unwrap();
        assert_eq!(abnormal_player.faction_type, Factions::Alien);
        assert_eq!(abnormal_player.player_name, "ßЉбббппѐѐ");
        assert_eq!(abnormal_player.structure_kill[0], 1);
        assert_eq!(abnormal_player.unit_kill[1], 1);
        assert_eq!(abnormal_player.unit_kill[2], -1);
        assert_eq!(abnormal_player.points, -30);
    }

    #[test]
    fn human_vs_human_new_file() {
        let game = checking_file(Path::new("./test_stuff/L20241203.log"));
        assert_eq!(game.map, Maps::TheMaw);
    }
}
