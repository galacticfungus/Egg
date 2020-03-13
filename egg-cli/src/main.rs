use std::path;
use clap;
use std::fs;
use egg::Repository;

fn main() {

    println!("Hello, world!");
    let matches = parse_arguments();

    // You can also match on a sub-command's name
    match matches.subcommand() {
        ("init", Some(init_matches)) => {
            // Value is either set or is a default
            let to_init = init_matches.value_of("init_path").unwrap();
            let path_to_init = path::Path::new(to_init);
            if path_to_init.exists() == false {
                if let Err(error) = fs::create_dir_all(path_to_init) {
                    println!("Could not create the directory {} when initializing the repository, error was {}", path_to_init.display(), error);
                    return;
                }
            }
            if let Err(error) = egg::Repository::create_repository(path_to_init) {
                println!("Failed to create the repository, error was {}", error);
                return;
            }
        },
        ("snapshot", Some(snapshot_matches)) => {
            match snapshot_matches.subcommand() {
                ("files", Some(file_list)) => {
                    
                    // Process a list of files passed as arguments and wild cards
                    // If the file list argument is present then process it
                    
                        let files_to_include = file_list.values_of("snapshot_files_list").unwrap();
                        println!("Processing file list: {:?}", files_to_include);
                        let mut list_of_paths = Vec::new();
                        for file in files_to_include {
                            let path_to_file = path::Path::new(file);
                            if path_to_file.exists() ==false {
                                println!("The file {} does not exist, creating a snapshot failed", path_to_file.display());
                                // TODO: Also consider the case where the path points to a directory
                                return;
                            }
                            list_of_paths.push(path_to_file);
                        }
                        let mut path_iterator = list_of_paths.iter();
                        let first_path = path_iterator.next().unwrap();
                      let egg = match Repository::find_egg(first_path) {
                          Ok(egg) => egg,
                          Err(error) => unimplemented!("error handling for failing to find an egg, error was {}", error),
                      };
                    
                        // Now check each path to see that they are all within that repositories working directory
                        let working_path = egg.get_working_path();
                        // TODO: Is within working path?
                        println!("Working path is {}", working_path.display());
                      
                    
                },
                ("incremental", Some(file_list)) => {
                    // Snapshot based on most recent snapshot taken - ie same files in that snapshot are snapshotted again
                },
                ("tracked", Some(file_list)) => {
                    // Takes a snapshot of every file this is current being tracked by the repository, ie every file that has every been included in a snapshot
                },
                _ => panic!("Unknown snapshot command: {:?}", snapshot_matches)
            }
        },
            ("", None) => println!("No sub-command was used"),
        _           => println!("Some other sub-command was used"),
    }
}

fn parse_arguments<'a>() -> clap::ArgMatches<'a> {
    let matches = clap::App::new("Egg-Cli")
      .version("0.1")
      .subcommand(clap::SubCommand::with_name("init")
        .about("Initialize a repository")
        .version("0.1")
        .author("Someone E. <someone_else@other.com>")
        .arg(clap::Arg::with_name("init_path")
          .index(1)
          .help("path to create the repository in")
          .default_value(".")
          .takes_value(true)))
      .subcommand(clap::SubCommand::with_name("snapshot")
        .alias("s")
        .about("Take a snapshot of the working directory")
        .version("0.1")
        .author("Someone E. <someone_else@other.com>")
        .subcommand(clap::SubCommand::with_name("files")
          .alias("f")
          .about("Take a snapshot of the following files")
          .version("0.1")
          .author("Someone E. <someone_else@other.com>")
          .arg(clap::Arg::with_name("snapshot_files_list")
            .multiple(true)
            .min_values(1)
            .required(true)
            .help("path to create the repository in")
            .takes_value(true))))
      .arg(clap::Arg::with_name("tree")
        .short("t")
        .long("tree")
        .help("Displays a tree of snapshots")
        .takes_value(false)
      )
      .arg(clap::Arg::with_name("take")
        .short("a")
        .long("take")
        .help("takes a snapshot of currently tracked files")
        .takes_value(false))
      .arg(clap::Arg::with_name("track")
        .index(1)
        .help("takes a snapshot of currently tracked files")
        .takes_value(true)
        .value_name("filename_to_track"))
      .get_matches();
    matches
}
