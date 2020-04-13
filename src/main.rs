use std::env;
use clap::{Arg, App};
use serde::Deserialize;
use toml::Value;
use toml::value::Array;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use tico::tico;
use git2::{ Repository, Status };
use std::path::{Path, PathBuf};
use shellexpand;
use colored::*;
use std::borrow::Cow;
use std::io::{self, Write};

// struct Plugin {
//     name: String
// }

#[derive(Deserialize, Debug)]
struct Options {
    prompt_char: String
}

#[derive(Deserialize, Debug)]
struct Config {
    options: Options,
    plugins: Array,
}

fn main() {
    let matches = App::new("familiar")
        .version("0.0.1")
        .author("Wuz <sup@wuz.fyi>")
        .about("A dark magic bash prompt")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Set a custom config file")
            .takes_value(true)
        )
        .get_matches();
    let config_file_str = matches.value_of("config").unwrap_or("~/.config/familiar/familiar.toml");
    let config_file = shellexpand::full(config_file_str).unwrap();
    let f = File::open(config_file.to_string()).unwrap();
    let mut reader = BufReader::new(f);

    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    let config: Config = toml::from_str(&contents).unwrap();
    let prompt_char = config.options.prompt_char;
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    write!(handle, "{}", familiar(prompt_char)).unwrap();
}

fn replace_home_dir<'a>(path: &'a str) -> String {
    let home = env::var("HOME").unwrap();
    let home_dir = &home.clone();
    let home_dir_ext = format!("{}{}", home_dir, "/");
    match &path {
        p if p == home_dir => path.replacen(&home_dir[..], "~", 1),
        p if p.starts_with(&home_dir_ext) => path.replacen(&home_dir[..], "~", 1),
        _ => path.to_string()
    }
}

fn basename<'a>(path: &'a str, sep: char) -> (String, Cow<'a, str>) {
    let mut pieces = path.rsplit(sep);
    match pieces.next() {
        Some(p) => (pieces.filter(|&i|i != p).rev().collect::<Vec<&str>>().join("/"), p.into()),
        None => (path.into(), "".into()),
    }
}

fn cwd() -> Option<(String, String)> {
    let path_env = env::current_dir().unwrap();
    let mut path = format!("{}", path_env.display());
    path = replace_home_dir(&path).to_string();
    let short_path = tico(&path);
    let (rest_path, basename) = basename(&short_path, '/');
    return Some((rest_path, basename.to_string()));
}

fn git() -> Option<(colored::ColoredString, colored::ColoredString)> {
    let current_dir = env::var("PWD").unwrap();
    let mut repo: Option<Repository> = None;
    let current_path = Path::new(&current_dir[..]);
    for path in current_path.ancestors() {
        match Repository::open(path) {
            Ok(r) => {
                repo = Some(r);
                break;
            }
            Err(_) => {},
        }
    }
    if repo.is_none() {
        return None
    }
    let repo = repo.unwrap();
    let reference = match repo.head() {
        Ok(r) => r,
        Err(_) => return None
    };
    let branch;
    if reference.is_branch() {
        branch = format!("{}", reference.shorthand().unwrap());
    } else {
        let commit = reference.peel_to_commit().unwrap();
        let id = commit.id();
        branch = format!("{:.6}", id);
    }
    let stat_char = "·".into();
    let mut repo_stat = stat_char;
    let file_stats = repo.statuses(None).unwrap();
    for file in file_stats.iter() {
        match file.status() {
            // STATE: unstaged (working tree modified)
            Status::WT_NEW        | Status::WT_MODIFIED      |
                Status::WT_DELETED    | Status::WT_TYPECHANGE    |
                Status::WT_RENAMED => {
                    let stat_char = "×".red().into();
                    repo_stat = stat_char;
                    break;
                },
                // STATE: staged (changes added to index)
                Status::INDEX_NEW     | Status::INDEX_MODIFIED   |
                    Status::INDEX_DELETED | Status::INDEX_TYPECHANGE |
                    Status::INDEX_RENAMED => {
                        let stat_char = "±".yellow().into();
                        repo_stat = stat_char;
                    },
                    // STATE: committed (changes have been saved in the repo)
                _ => {}
        }
    }

    let color_branch = branch.blue().bold().italic();
    return Some((color_branch, repo_stat))
}

fn familiar(prompt_char: String) -> String {
    let (rest_path, basename) = cwd().unwrap_or(("".into(), "".into()));
    let (branch, status) = git().unwrap_or(("".into(), "".into()));
    let mut rest = rest_path;
    if rest != "" {
      rest = rest + "/";
    }
    format!(
        "{rest_path}{basename} {branch} {status} \n{pchar} ",
        rest_path = rest.black().bold(),
        basename = basename.cyan(),
        branch = branch,
        status = status,
        pchar = prompt_char
    )
}
