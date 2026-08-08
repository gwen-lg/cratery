/*******************************************************************************
 * Copyright (c) 2021 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

use std::{env, process::Command};

fn main() {
    let db_url = "sqlite://src/empty.db";
    println!("cargo:rustc-env=DATABASE_URL={db_url}");

    if let Some(git_hash) = git_hash() {
        println!("cargo:rustc-env=GIT_HASH={git_hash}");
    }
    if let Some(git_tag) = git_tag() {
        println!("cargo:rustc-env=GIT_TAG={git_tag}");
    }
}

fn git_hash() -> Option<String> {
    env::var("GIT_HASH").ok().or_else(|| {
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .map(|output| String::from_utf8(output.stdout).unwrap_or_default().trim().to_string())
    })
}

fn git_tag() -> Option<String> {
    env::var("GIT_TAG").ok().or_else(|| {
        Command::new("git")
            .args(["tag", "-l", "--points-at", "HEAD"])
            .output()
            .ok()
            .map(|output| String::from_utf8(output.stdout).unwrap_or_default().trim().to_string())
    })
}
