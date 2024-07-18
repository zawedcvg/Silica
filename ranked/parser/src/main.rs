pub mod inserting_info;
use dotenv::dotenv;
use sqlx::postgres::{PgPoolOptions, Postgres};
use std::fs::OpenOptions;
use std::io::{self};
use std::sync::Arc;
use std::time::Instant;
pub mod parser;
use crate::inserting_info::inserting_info;
use crate::parser::checking_folder;
use std::env;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(windows)]
use {
    std::os::windows::io::AsRawHandle,
    winapi::um::handleapi::INVALID_HANDLE_VALUE,
    winapi::um::processenv::SetStdHandle,
    winapi::um::winbase::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE},
};

#[tokio::main]
async fn main() {
    // Open files to append stdout and stderr
    //let stdout_file = OpenOptions::new()
    //.create(true)
    //.append(true)
    //.open("stdout.txt")
    //.unwrap_or_else(|err| panic!("Failed to open stdout.txt: {}", err));
    //let stderr_file = OpenOptions::new()
    //.create(true)
    //.append(true)
    //.open("stderr.txt")
    //.unwrap_or_else(|err| panic!("Failed to open stderr.txt: {}", err));

    //#[cfg(unix)]
    //{
    //// Redirect stdout on Unix
    //unsafe {
    //let stdout_fd = libc::dup(1);
    //if stdout_fd == -1 {
    //panic!("Failed to duplicate stdout: {}", io::Error::last_os_error());
    //}
    //if libc::dup2(stdout_file.as_raw_fd(), 1) == -1 {
    //libc::dup2(stdout_fd, 1);
    //libc::close(stdout_fd);
    //panic!("Failed to redirect stdout: {}", io::Error::last_os_error());
    //}
    //libc::close(stdout_fd);
    //}

    //// Redirect stderr on Unix
    //unsafe {
    //let stderr_fd = libc::dup(2);
    //if stderr_fd == -1 {
    //panic!("Failed to duplicate stderr: {}", io::Error::last_os_error());
    //}
    //if libc::dup2(stderr_file.as_raw_fd(), 2) == -1 {
    //libc::dup2(stderr_fd, 2);
    //libc::close(stderr_fd);
    //panic!("Failed to redirect stderr: {}", io::Error::last_os_error());
    //}
    //libc::close(stderr_fd);
    //}
    //}

    //#[cfg(windows)]
    //{
    //use std::ptr::null_mut;

    //// Redirect stdout on Windows
    //unsafe {
    //let stdout_handle = stdout_file.as_raw_handle();
    //if stdout_handle == INVALID_HANDLE_VALUE as _ {
    //panic!(
    //"Invalid handle for stdout.txt: {}",
    //io::Error::last_os_error()
    //);
    //}
    //if SetStdHandle(STD_OUTPUT_HANDLE, stdout_handle as _) == 0 {
    //panic!(
    //"Failed to set stdout handle: {}",
    //io::Error::last_os_error()
    //);
    //}
    //}

    //// Redirect stderr on Windows
    //unsafe {
    //let stderr_handle = stderr_file.as_raw_handle();
    //if stderr_handle == INVALID_HANDLE_VALUE as _ {
    //panic!(
    //"Invalid handle for stderr.txt: {}",
    //io::Error::last_os_error()
    //);
    //}
    //if SetStdHandle(STD_ERROR_HANDLE, stderr_handle as _) == 0 {
    //panic!(
    //"Failed to set stderr handle: {}",
    //io::Error::last_os_error()
    //);
    //}
    //}
    //}
    //Warm up/start the connection pool here
    let now = Instant::now();
    let args: Vec<String> = env::args().collect();

    dotenv().ok();

    // Get the database URL from the environment variable
    let database_url = env::var("DATABASE_URL").unwrap();
    println!("{}", database_url);
    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url);
    let game = checking_folder(&args[1]);
    println!("{:?} time elapsed for everything", now.elapsed());
    let _ = inserting_info(
        Arc::new(game),
        pool.await
            .unwrap_or_else(|e| panic!("something went wrong due to {e}")),
    )
    .await;
}
