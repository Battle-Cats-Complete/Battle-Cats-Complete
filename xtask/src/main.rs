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

fn run_cargo(args: &[&str]) {
    let status = Command::new("cargo")
        .args(args)
        .status()
        .expect("Failed to execute cargo command");

    if !status.success() {
        eprintln!("Error: 'cargo {}' failed. Aborting release.", args.join(" "));
        exit(1);
    }
}

fn is_lockfile_dirty() -> bool {
    let output = Command::new("git")
        .args(["status", "--porcelain", "Cargo.lock"])
        .output()
        .expect("Failed to execute git status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    !stdout.trim().is_empty()
}

fn main() {
    println!("Starting BCC Release...");

    println!("> Pulling latest nyanko commits...");
    run_cargo(&["update", "-p", "nyanko"]);
    run_cargo(&["clean", "-p", "nyanko"]);

    println!("> Verifying build state with cargo check...");
    run_cargo(&["check"]);

    if is_lockfile_dirty() {
        println!("> Cargo.lock modified by update. Committing...");
        run_git(&["add", "Cargo.lock"]);
        run_git(&["commit", "--amend", "--no-edit"]);

        println!("> Syncing amended history to remote nightly...");
        run_git(&["push", "origin", "nightly", "--force-with-lease"]);
    } else {
        println!("> Nyanko is already up-to-date. No lockfile changes.");
    }

    println!("> Checking out main and pulling latest changes...");
    run_git(&["checkout", "main"]);
    run_git(&["pull"]);

    println!("> Merging nightly into main...");
    run_git(&["merge", "nightly"]);

    println!("> Pushing main to remote...");
    run_git(&["push"]);

    println!("> Returning to nightly branch...");
    run_git(&["checkout", "nightly"]);

    println!("Release successful! Branches are even and dependencies are up-to-date.");
}