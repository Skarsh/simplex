use std::error::Error;
use std::fmt;

#[derive(Debug)]
enum SimplexError {
    MissingCommand,
    MissingInstallPackage,
    MissingRemovePackage,
    MissingInfoPackage,
    IllegalCommand,
}

impl fmt::Display for SimplexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SimplexError::MissingCommand => write!(f, "User did not specify command. Usage: simplex <command> [<args>]"),
            SimplexError::MissingInstallPackage => write!(f, "User did not specify which package to be installed. Usage: simplex install <package-name>"),
            SimplexError::IllegalCommand => write!(f, "User specified an illegal command. For more info about legal commands: simplex --help"),
            SimplexError::MissingRemovePackage => write!(f, "User did not specify which package to be removed. Usage: simplex remove <package-name>"),
            SimplexError::MissingInfoPackage => write!(f, "User did not specify which package to get more information about. Usage: simplex info <package-name>"),
        }
    }
}

impl Error for SimplexError {}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(Box::new(SimplexError::MissingCommand));
    }

    match args[1].as_str() {
        "install" => {
            if args.len() < 3 {
                return Err(Box::new(SimplexError::MissingInstallPackage));
            }
            println!("Installing package {} ...", args[2]);
        },
        "remove" => {
            if args.len() < 3 {
                return Err(Box::new(SimplexError::MissingRemovePackage));
            }
            println!("Removing package {} ...", args[2]);
        },
        "list" => {
            println!("Listing packages...")
        },
        "info" => {
            if args.len() < 3 {
                return Err(Box::new(SimplexError::MissingInfoPackage));
            }
            println!("Info about package {} ...", args[2]);
        }
        _ => return Err(Box::new(SimplexError::IllegalCommand)),
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
