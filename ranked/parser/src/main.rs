pub mod inserting_info;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
pub mod parser;
use crate::inserting_info::inserting_info;
use crate::parser::checking_folder;
use std::env;

fn main() {

    let stdout_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("stdout.txt")
        .unwrap_or_else(|e| panic!("unable to create stdout file due to {e}"));

    let stderr_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("stderr.txt")
        .unwrap_or_else(|e| panic!("unable to create stdout file due to {e}"));

    // Redirect stdout dont really know what's going on
    unsafe {
        let stdout_fd = libc::dup(1);
        libc::dup2(stdout_file.as_raw_fd(), 1);
        libc::close(stdout_fd);
    }

    // Redirect stderr
    unsafe {
        let stderr_fd = libc::dup(2);
        libc::dup2(stderr_file.as_raw_fd(), 2);
        libc::close(stderr_fd);
    }

    let args: Vec<String> = env::args().collect();
    let game = checking_folder(&args[1]);
    let _ = inserting_info(game);
}
