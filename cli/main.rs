use std::env;
use std::path::PathBuf;
use std::process;

mod ast_log;
mod build;
mod project;
mod server;
mod validation;

struct PathCommandOptions {
    path: PathBuf,
    log_ast: bool,
    debug: bool,
}

fn print_help() {
    println!(
        "HRML - Hypertext Rust Markup Language v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Usage: hrml <command> [options]");
    println!();
    println!("Commands:");
    println!("  new <name>          Create a new HRML project");
    println!("  dev [path]          Run development server with auto-reload");
    println!("  serve [path]        Run production server");
    println!("  build [path]        Build static site for deployment");
    println!("  check [path]        Validate templates and configuration");
    println!("  convert [file]      Convert .hrml to .trml syntax");
    println!("  --log-ast           Regenerate ast.log before build/serve/dev");
    println!("  --debug, -d         Enable verbose render diagnostics");
    println!("  version             Show version information");
    println!("  help                Show this help message");
    println!();
    println!("Examples:");
    println!("  hrml new myapp              Create new project 'myapp'");
    println!("  hrml dev                    Start dev server in current directory");
    println!("  hrml serve ./myapp          Serve project from ./myapp");
    println!("  hrml build ./myapp          Build static site from ./myapp");
    println!("  hrml check                  Validate current project");
}

fn parse_path_command_options(args: &[String]) -> Result<PathCommandOptions, String> {
    let mut path = None;
    let mut log_ast = false;
    let mut debug = false;

    for arg in args {
        if arg == "--log-ast" {
            log_ast = true;
        } else if arg == "--debug" || arg == "-d" {
            debug = true;
        } else if arg.starts_with('-') {
            return Err(format!("Unknown option: {}", arg));
        } else if path.is_none() {
            path = Some(PathBuf::from(arg));
        } else {
            return Err(format!("Unexpected extra argument: {}", arg));
        }
    }

    Ok(PathCommandOptions {
        path: path.unwrap_or_else(|| PathBuf::from(".")),
        log_ast,
        debug,
    })
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    let command = &args[1];

    match command.as_str() {
        "help" | "--help" | "-h" => print_help(),
        "version" | "--version" | "-v" => println!("HRML {}", env!("CARGO_PKG_VERSION")),
        "new" => {
            if args.len() < 3 {
                eprintln!("Error: Project name required");
                eprintln!("Usage: hrml new <name>");
                process::exit(1);
            }

            let name = &args[2];
            println!("Creating new HRML project: {}", name);
            if let Err(e) = project::create_project(name) {
                eprintln!("Error creating project: {}", e);
                process::exit(1);
            }

            println!("Project '{}' created successfully!", name);
            println!();
            println!("To get started:");
            println!("  cd {}", name);
            println!("  hrml dev");
        }
        "dev" => {
            let options = match parse_path_command_options(&args[2..]) {
                Ok(options) => options,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            if let Err(e) = server::run_dev(&options.path, options.log_ast, options.debug).await {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        "serve" => {
            let options = match parse_path_command_options(&args[2..]) {
                Ok(options) => options,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            if let Err(e) = server::serve_static(&options.path, options.log_ast).await {
                eprintln!("Serve error: {}", e);
                process::exit(1);
            }
        }
        "build" => {
            let options = match parse_path_command_options(&args[2..]) {
                Ok(options) => options,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            if let Err(e) = build::build_site(&options.path, options.log_ast) {
                eprintln!("Build failed: {}", e);
                process::exit(1);
            }
        }
        "check" => {
            let options = match parse_path_command_options(&args[2..]) {
                Ok(options) => options,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            if options.log_ast {
                let config = match project::load_project_config(&options.path) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Check failed: {}", e);
                        process::exit(1);
                    }
                };
                if let Err(e) = ast_log::write_ast_log(&options.path, &config) {
                    eprintln!("Check failed: {}", e);
                    process::exit(1);
                }
            }
            if let Err(e) = project::check_project(&options.path) {
                eprintln!("Check failed: {}", e);
                process::exit(1);
            }
        }
        "convert" => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| { eprintln!("Read error: {e}"); process::exit(1); });
            match xrml::convert::to_trml(&source) {
                Ok(trml) => println!("{trml}"),
                Err(e) => { eprintln!("Convert error: {e}"); process::exit(1); }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!();
            print_help();
            process::exit(1);
        }
    }
}
