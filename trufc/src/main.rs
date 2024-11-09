mod build_sys;
mod config;
mod constants;
mod utils;

use std::{env, fs, process};

use clap::{Parser, Subcommand};
use config::Config;
use constants::CONFIG_FILE;

#[derive(Parser, Debug)]
#[command(name = "TrufC")]
#[command(version = "0.0.3")]
#[command(about = "A build system that integrates with truffle optimizations.", long_about = None)]
struct CliCommand {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init {
        #[arg(value_enum, long, default_value = "c")]
        language: utils::Language,
    },
    New {
        proj_name: String,

        #[arg(value_enum, long, default_value = "c")]
        language: utils::Language,
    },

    // Clap doesn't provide any way to structure the syntax to be `trufc run --profile
    // So, we'll have to paese these manually.
    Build {
        profile: String,
    },
    Run {
        profile: String,
        args: Vec<String>,
    },
}

impl Commands {
    fn new(variant: &str, profile: &str, args: Vec<String>) -> Self {
        match variant {
            "build" => Self::Build {
                profile: profile.to_string(),
            },
            "run" => Self::Run {
                profile: profile.to_string(),
                args,
            },
            _ => panic!("Parameter `variant` must be one of 'build' or 'run'"),
        }
    }
}

fn main() {
    let cli: CliCommand;

    let raw_cli_args = std::env::args().collect::<Vec<String>>();

    if raw_cli_args.len() < 2 {
        // Let the program fail and have Clap display it's help message
        cli = CliCommand::parse();
    } else if raw_cli_args[1] == "run" || raw_cli_args[1] == "build" {
        let mut profile = "--dev".to_string();
        let mut args = vec![];
        if raw_cli_args.len() >= 3 && raw_cli_args[2].starts_with("--") && raw_cli_args.len() > 2 {
            profile = raw_cli_args[2].clone();
        }
        if let Some(idx) = raw_cli_args.iter().position(|i| i == "--") {
            assert!([2_usize, 3_usize].contains(&idx));
            args = raw_cli_args[(idx + 1)..].to_vec();
        } else {
            assert!(raw_cli_args.len() <= 3);
        }

        cli = CliCommand {
            command: Commands::new(&raw_cli_args[1], &profile, args),
        }
    } else {
        cli = CliCommand::parse();
    }

    match cli.command {
        Commands::Init { language } => {
            let cwd = env::current_dir().unwrap();

            if let Err(e) = build_sys::create_project(&cwd, language) {
                println!("An error occurred while creating the project:\n{}", e);
                process::exit(1);
            }
        }
        Commands::New {
            proj_name,
            language,
        } => {
            let mut target_dir = env::current_dir().unwrap();
            target_dir.push(proj_name);
            if target_dir.exists() {
                println!("Error: file of directory already exists");
                process::exit(1);
            }
            fs::create_dir(&target_dir).unwrap();

            if let Err(e) = build_sys::create_project(&target_dir, language) {
                println!("An error occurred while creating the project:\n{}", e);
            }
        }
        Commands::Build { profile } => {
            handle_build(profile);
        }
        Commands::Run { profile, args } => {
            handle_build(profile.clone());

            let mut cwd = env::current_dir().unwrap();

            cwd.push(CONFIG_FILE);
            let config = Config::from(&cwd).unwrap();
            cwd.pop();

            cwd.push("build");
            cwd.push(&profile[2..]);
            cwd.push(config.project.name);

            let bin = cwd.to_str().unwrap();

            // Spawn the compiled binary and pass any arguments if necessary
            let mut child_builder = process::Command::new(bin);
            let child: &mut process::Command;
            if args.len() > 0 {
                child = child_builder.args(&args);
            } else {
                child = &mut child_builder;
            }
            let mut child = child.spawn().unwrap();
            child.wait().unwrap();
        }
    }
}

fn handle_build(profile: String) {
    if !profile.starts_with("--") {
        println!("Error: profile must start with `--`");
        process::exit(1);
    }

    let mut cwd = env::current_dir().unwrap();
    cwd.push("build");
    cwd.push(&profile[2..]);
    if !cwd.exists() {
        fs::create_dir_all(&cwd).unwrap();
    }
    cwd.pop();
    cwd.pop();

    cwd.push(CONFIG_FILE);
    let config = Config::from(&cwd).unwrap();
    cwd.pop();

    let link_file = build_sys::link_files(&cwd);
    let link_lib = build_sys::link_lib(&cwd);
    let opt_flags = build_sys::opt_flags(&profile, &config).unwrap();

    let compilation_cmd =
        build_sys::full_compilation_cmd(&config, &profile, &link_file, &link_lib, &opt_flags)
            .unwrap();

    let mut child = process::Command::new(&compilation_cmd[0])
        .args(&compilation_cmd[1..])
        .spawn()
        .unwrap();

    child.wait().unwrap();
}
