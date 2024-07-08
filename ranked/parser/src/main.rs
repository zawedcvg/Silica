pub mod inserting_info;
pub mod parser;
use crate::inserting_info::inserting_info;
use crate::parser::checking_folder;

fn main() {
    let game = checking_folder("/home/neeladri/Silica/ranked/log_folder/".to_string());
    let _ = inserting_info(game);
}
