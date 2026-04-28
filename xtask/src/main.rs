use std::process::{Command, exit};

fn run_git(args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .status()
        .expect("Failed to execute git command");

    if !status.success() {
        eprintln!("Error: 'git {}' failed. Aborting release.", args.join(" "));
        exit(1);
    }
}

fn main() {
    println!("Starting BCC Release...");

    println!("> Checking out main and pulling latest changes...");
    run_git(&["checkout", "main"]);
    run_git(&["pull"]);

    println!("> Merging nightly into main...");
    run_git(&["merge", "nightly"]);

    println!("> Pushing main to remote...");
    run_git(&["push"]);

    println!("> Returning to nightly branch...");
    run_git(&["checkout", "nightly"]);

    println!("Release successful! Branches are even.");
}