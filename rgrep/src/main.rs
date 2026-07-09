use regex::Regex;
use std::env;
use std::fs;
use walkdir::WalkDir;

fn search(contents: &str, regex: &Regex, invert: bool) -> Vec<(usize, String)> {
    let mut results: Vec<(usize, String)> = Vec::new();
    let mut line_number = 1;

    for line in contents.lines() {
        let is_match = regex.is_match(line);

        let should_include = if invert { !is_match } else { is_match };

        if should_include {
            results.push((line_number, line.to_string()));
        }

        line_number += 1;
    }
    results
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: rgrep [OPTIONS] <pattern> <path>");
        eprintln!("Options: -v -c -l");
        std::process::exit(1);
    }

    let mut invert = false;
    let mut count_only = false;
    let mut files_only = false;
    let mut pattern_index = 1;

    for i in 1..args.len() {
        match args[i].as_str() {
            "-v" => invert = true,
            "-c" => count_only = true,
            "-l" => files_only = true,
            _ => {
                pattern_index = i;
                break;
            }
        }
    }

    if pattern_index + 1 >= args.len() {
        eprintln!("Error: missing pattern or path");
        std::process::exit(1);
    }

    let pattern = &args[pattern_index];
    let path = &args[pattern_index + 1];

    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Invalid pattern: {}", e);
            std::process::exit(1);
        }
    };

    for entry in WalkDir::new(path).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path();

        let contents = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let matches = search(&contents, &regex, invert);
        if files_only {
            if !matches.is_empty() {
                println!("{}", file_path.display());
            }
            continue;
        }

        if count_only {
            println!("{}: {}", file_path.display(), matches.len());
            continue;
        }

        for i in 0..matches.len() {
            let line_num = matches[i].0;
            let line = &matches[i].1;
            println!("{}:{}: {}", file_path.display(), line_num, line);
        }
    }
}
