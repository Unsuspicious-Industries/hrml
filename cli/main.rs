use std::env;
use std::path::PathBuf;
use std::process;

mod ast_log;
mod build;
mod palette;
mod project;
mod server;
mod validation;

struct PathCommandOptions {
    path: PathBuf,
    log_ast: bool,
    debug: bool,
    palette: Option<PathBuf>,
    host: Option<String>,
    port: Option<u16>,
}

fn print_help() {
    println!(
        "HRML - Hypertext Rust Markup Language v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    let bin = env!("CARGO_BIN_NAME");
    println!("Usage: {bin} <command> [options]");
    println!();
    println!("Commands:");
    println!("  new <name>          Create a new HRML project");
    println!("  dev [path]          Run development server with auto-reload");
    println!("  serve [path]        Run production server from the project source");
    println!("  build [path]        Build static site for deployment");
    println!("  check [path]        Validate templates and configuration");
    println!("  convert [file]      Convert .hrml to .trml syntax");
    println!();
    println!("Options:");
    println!("  --log-ast           Regenerate ast.log before build/serve/dev");
    println!("  --debug, -d         Enable verbose render diagnostics");
    println!("  --palette <file>    Resolve USI_<ROLE> tokens from a TOML palette file");
    println!("  --host <address>    Override the bind address (serve)");
    println!("  --port <number>     Override the bind port (serve)");
    println!("  version             Show version information");
    println!("  help                Show this help message");
    println!();
    println!("Examples:");
    println!("  {bin} new myapp              Create new project 'myapp'");
    println!("  {bin} dev                    Start dev server in current directory");
    println!("  {bin} serve ./myapp          Serve project from ./myapp");
    println!("  {bin} build ./myapp          Build static site from ./myapp");
    println!("  {bin} build --palette ./palette.toml ./myapp");
    println!("  {bin} check                  Validate current project");
}

fn parse_path_command_options(args: &[String]) -> Result<PathCommandOptions, String> {
    let mut path = None;
    let mut log_ast = false;
    let mut debug = false;
    let mut palette = None;
    let mut host = None;
    let mut port = None;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--log-ast" => log_ast = true,
            "--debug" | "-d" => debug = true,
            "--palette" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or("Option --palette requires a file path")?;
                palette = Some(PathBuf::from(value));
            }
            "--host" => {
                i += 1;
                let value = args.get(i).ok_or("Option --host requires an address")?;
                host = Some(value.clone());
            }
            "--port" => {
                i += 1;
                let value = args.get(i).ok_or("Option --port requires a number")?;
                port = Some(
                    value
                        .parse::<u16>()
                        .map_err(|_| format!("Invalid port: {}", value))?,
                );
            }
            _ if arg.starts_with('-') => return Err(format!("Unknown option: {}", arg)),
            _ if path.is_none() => path = Some(PathBuf::from(arg)),
            _ => return Err(format!("Unexpected extra argument: {}", arg)),
        }
        i += 1;
    }

    Ok(PathCommandOptions {
        path: path.unwrap_or_else(|| PathBuf::from(".")),
        log_ast,
        debug,
        palette,
        host,
        port,
    })
}

fn load_palette(path: &Option<PathBuf>) -> Result<Option<palette::Palette>, String> {
    match path {
        Some(p) => palette::Palette::load(p).map(Some),
        None => Ok(None),
    }
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
                eprintln!("Usage: {} new <name>", env!("CARGO_BIN_NAME"));
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
            println!("  {} dev", env!("CARGO_BIN_NAME"));
        }
        "dev" => {
            let options = match parse_path_command_options(&args[2..]) {
                Ok(options) => options,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            let palette = match load_palette(&options.palette) {
                Ok(palette) => palette,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            if let Err(e) =
                server::run_dev(&options.path, palette.as_ref(), options.log_ast, options.debug)
                    .await
            {
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
            let palette = match load_palette(&options.palette) {
                Ok(palette) => palette,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            if let Err(e) = server::run_serve(
                &options.path,
                palette.as_ref(),
                options.host,
                options.port,
                options.log_ast,
            )
            .await
            {
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
            let palette = match load_palette(&options.palette) {
                Ok(palette) => palette,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            if let Err(e) = build::build_site(&options.path, options.log_ast, palette.as_ref()) {
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
            let palette = match load_palette(&options.palette) {
                Ok(palette) => palette,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            if options.log_ast {
                let config = match project::load_project_config(
                    &options.path,
                    palette.as_ref(),
                ) {
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
            if let Err(e) = project::check_project(&options.path, palette.as_ref()) {
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
