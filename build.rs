/*******************************************************************************
 * Copyright (c) 2021 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

use std::{
    env,
    process::{Command, Output},
};

fn main() {
    let db_url = "sqlite://src/empty.db";
    println!("cargo:rustc-env=DATABASE_URL={db_url}");

    extract_and_set_env("GIT_HASH", "git", &["rev-parse", "HEAD"]);
    extract_and_set_env("GIT_TAG", "git", &["tag", "-l", "--points-at", "HEAD"]);
}

fn extract_and_set_env(env_var: &str, cmd: &str, args: &[&str]) {
    if let Some(git_hash) = extract_from_env_or_git(env_var, cmd, args) {
        println!("cargo:rustc-env={env_var}={git_hash}");
    }
}

fn extract_from_env_or_git(env_var: &str, cmd: &str, args: &[&str]) -> Option<String> {
    env::var(env_var)
        .ok()
        .or_else(|| Command::new(cmd).args(args).output().extract_output_to_string())
}

trait OptionOutputExt {
    fn extract_output_to_string(self) -> Option<String>;
}

impl<E> OptionOutputExt for Result<Output, E> {
    fn extract_output_to_string(self) -> Option<String> {
        self.ok()
            .and_then(|output| String::from_utf8(output.stdout).ok().map(|out| out.trim().to_string()))
    }
}
