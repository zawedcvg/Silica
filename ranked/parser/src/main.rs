pub mod inserting_info;
pub mod parser;
use crate::inserting_info::inserting_info;
use std::env;
use crate::parser::checking_folder;


fn main() {
    let args: Vec<String> = env::args().collect();
    let game = checking_folder(&args[1]);
    let _ = inserting_info(game);
}
