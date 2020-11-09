use atty::Stream;
use clap::{load_yaml, App};
use colored::{self, Colorize};
use nomino::errors::SourceError;
use nomino::input::{Context, Formatter, Source};
use prettytable::{cell, format, row, Table};
use serde_json::map::Map;
use serde_json::value::Value;
use std::env::{args, set_current_dir};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::exit;

fn read_source(
    regex: Option<&str>,
    sort: Option<&str>,
    map: Option<&str>,
) -> Result<Source, Box<dyn Error>> {
    match (regex, sort, map) {
        (Some(pattern), _, _) => Source::new_regex(pattern),
        (_, Some(order), _) => Source::new_sort(order),
        (_, _, Some(filename)) => Source::new_map(filename),
        _ => {
            colored::control::set_override(atty::is(Stream::Stderr));
            Err(Box::new(SourceError::new(format!(
                "one of '{}', '{}' or '{}' options must be set.\n{}: run '{} {}' for more information.",
                "regex".cyan(),
                "sort".cyan(),
                "map".cyan(),
                "usage".yellow().bold(),
                args().next().unwrap().cyan(),
                "--help".cyan(),
            ))))
        }
    }
}

fn read_output(output: Option<&str>) -> Result<Option<Formatter>, Box<dyn Error>> {
    if output.is_none() {
        return Ok(None);
    }
    Ok(Some(Formatter::new(output.unwrap())?))
}

fn rename_files(
    context: Context,
    test_mode: bool,
    need_map: bool,
    overwrite: bool,
    mkdir: bool,
) -> Result<(Option<Map<String, Value>>, bool), Box<dyn Error>> {
    let mut map = if need_map { Some(Map::new()) } else { None };
    let mut is_renamed = true;
    let mut with_err = false;
    for (input, mut output) in context {
        let mut file_path_buf;
        let mut file_path = Path::new(output.as_str());
        if !overwrite {
            while file_path.exists() {
                file_path_buf = file_path
                    .with_file_name(
                        (String::from("_")
                            + file_path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or_default())
                        .as_str(),
                    )
                    .to_path_buf();
                file_path = file_path_buf.as_path();
                output = file_path.to_string_lossy().to_string();
            }
        }
        if mkdir {
            let _ = file_path
                .parent()
                .and_then(|parent| fs::create_dir_all(parent).ok());
        }
        if !test_mode {
            match fs::rename(input.as_str(), file_path) {
                Ok(_) => is_renamed = true,
                Err(e) => {
                    is_renamed = false;
                    with_err = true;
                    eprintln!(
                        "[{}] unable to rename '{}': {}",
                        "error".red().bold(),
                        input.as_str(),
                        e.to_string()
                    );
                }
            }
        }
        if is_renamed && need_map {
            map.as_mut().map(|m| m.insert(output, Value::String(input)));
        }
        is_renamed = true;
    }
    Ok((map, with_err))
}

fn print_map_table(map: Map<String, Value>) -> Result<(), Box<dyn Error>> {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["Input".cyan(), "Output".cyan()]);
    map.into_iter()
        .enumerate()
        .for_each(|(i, (output, input))| {
            if let Value::String(input) = input {
                if i % 2 == 0 {
                    table.add_row(row![input.as_str(), output.as_str()]);
                } else {
                    table.add_row(row![input.as_str().purple(), output.as_str().purple()]);
                }
            }
        });
    table.printstd();
    Ok(())
}

fn run_app() -> Result<bool, Box<dyn Error>> {
    let opts_format = load_yaml!("opts.yml");
    let opts = App::from_yaml(opts_format).get_matches();
    if let Some(cwd) = opts.value_of("directory").map(Path::new) {
        set_current_dir(cwd)?;
    }
    let context = Context::new(
        read_source(
            opts.value_of("regex"),
            opts.value_of("sort"),
            opts.value_of("map"),
        )?,
        read_output(opts.value_of("output"))?,
        opts.is_present("extension"),
    )?;
    let print_map = opts.is_present("print");
    let generate_map = opts.value_of("generate");
    let (map, with_err) = rename_files(
        context,
        opts.is_present("test"),
        print_map || generate_map.is_some(),
        opts.is_present("overwrite"),
        opts.is_present("mkdir"),
    )?;
    if let Some(map_file) = generate_map {
        fs::write(
            map_file,
            serde_json::to_vec_pretty(map.as_ref().unwrap())?.as_slice(),
        )?;
    }
    if print_map && !map.as_ref().unwrap().is_empty() {
        colored::control::set_override(atty::is(Stream::Stdout));
        print_map_table(map.unwrap())?;
    }
    Ok(with_err)
}

fn main() {
    exit(match run_app() {
        Ok(with_err) => {
            if with_err {
                1
            } else {
                0
            }
        }
        Err(err) => {
            colored::control::set_override(atty::is(Stream::Stderr));
            eprintln!("{}: {}", "error".red().bold(), err.to_string());
            1
        }
    });
}
