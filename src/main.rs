use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
struct Package {
    name: String,
    version: String,
    dependencies: Vec<String>,
    build_steps: Vec<String>,
    url: String,
}

struct PackageManager {
    installed_packages: HashMap<String, Package>,
    store_path: PathBuf,
}

impl PackageManager {
    pub fn new(store_path: &str) -> Result<Self, Box<dyn Error>> {
        let absolute_store_path = if Path::new(store_path).is_absolute() {
            PathBuf::from(store_path)
        } else {
            env::current_dir()?.join(store_path)
        };

        let mut pm = PackageManager {
            installed_packages: HashMap::new(),
            store_path: absolute_store_path,
        };
        pm.create_directory_structure()?;
        pm.sync_installed_packages()?;

        Ok(pm)
    }

    fn create_directory_structure(&self) -> Result<(), Box<dyn Error>> {
        let dirs = [
            &self.store_path,
            &self.store_path.join("downloads"),
            &self.store_path.join("builds"),
            &self.store_path.join("installed"),
        ];

        for dir in &dirs {
            fs::create_dir_all(dir)?;
            println!("Created directory: {}", dir.display());
        }

        Ok(())
    }

    fn sync_installed_packages(&mut self) -> Result<(), Box<dyn Error>> {
        self.installed_packages.clear();
        let installed_dir = self.store_path.join("installed");
        for entry in fs::read_dir(installed_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name_version) = path.file_name() {
                    let name_version = name_version.to_string_lossy();
                    if let Some((name, version)) = name_version.rsplit_once('-') {
                        let package = Package {
                            name: name.to_string(),
                            version: version.to_string(),
                            dependencies: vec![],   // We would need to store this info somewhere
                            build_steps: vec![],    // Same here
                            url: String::new(),     // And here
                        };
                        self.installed_packages.insert(name_version.to_string(), package);
                    }
                }
            }
        }
            
        Ok(())
    }

    fn fetch_package(&self, package: &Package) -> Result<(), Box<dyn Error>> {
        println!("Fetching package: {}", package.name);
        let download_dir = Path::new(&self.store_path).join("downloads");
        fs::create_dir_all(&download_dir)?;
        env::set_current_dir(&download_dir)?;

        let output = Command::new("curl").args(&["-LO", &package.url]).output()?;
        if !output.status.success() {
            return Err(format!("Failed to download package: {}", package.name).into());
        }

        Ok(())
    }

    fn build_package(&self, package: &Package) -> Result<(), Box<dyn Error>> {
        println!("Building package: {}", package.name);

        // Create and move to build directory
        let build_dir =
            Path::new(&self.store_path).join(format!("{}-{}-build", package.name, package.version));
        fs::create_dir_all(&build_dir)?;
        env::set_current_dir(&build_dir)?;

        // Extract the source
        // TODO(Thomas): Deal with unwraps
        let source_tarball = Path::new(&package.url)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        let download_dir = Path::new(&self.store_path).join("downloads");
        let tarball_path = download_dir.join(source_tarball);

        Command::new("tar")
            .args(&["xzf", tarball_path.to_str().unwrap()])
            .status()?;

        // Move into the extracted directory
        let source_dir = build_dir.join(format!("{}-autoconf-3360000", package.name));
        env::set_current_dir(source_dir)?;

        // Modify build steps to use our store path
        let install_path = Path::new(&self.store_path)
            .join("installed")
            .join(format!("{}-{}", package.name, package.version));
        let modified_build_steps: Vec<String> = package
            .build_steps
            .iter()
            .map(|step| {
                if step.starts_with("./configure") {
                    format!("{} --prefix={}", step, install_path.to_str().unwrap())
                } else {
                    step.clone()
                }
            })
            .collect();

        // Execute build steps
        for step in &modified_build_steps {
            println!("Executing: {}", step);
            let output = Command::new("sh").arg("-c").arg(step).output()?;

            if !output.status.success() {
                return Err(format!(
                    "Build step failed: {}\nOutput: {}",
                    step,
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }

        // Return to original directory
        env::set_current_dir(Path::new(&self.store_path))?;

        Ok(())
    }

    fn install_package(&mut self, package: &Package) -> Result<(), Box<dyn Error>> {
        println!("Instsalling package: {}", package.name);
        let install_path = self.store_path.join("installed").join(format!("{}-{}", package.name, package.version));

        // The `make install` step should have already installed the package to our custom prefix
        // We just need to record that it's installed
        self.installed_packages.insert(package.name.clone(), package.clone());
        println!("Package installed to: {}", install_path.display());

        Ok(())
    }

    fn remove_package(&mut self, name: &str, version: &str) -> Result<(), Box<dyn Error>> {
        let key = format!("{}-{}", name, version);
        if self.installed_packages.remove(&key).is_some() {
            let install_path = self.store_path.join("installed").join(&key);
            println!("Removing package: {} {}", name, version);
            fs::remove_dir_all(install_path)?;
            Ok(())
        } else {
            Err(format!("Package not found: {} {}", name, version).into())
        }

    }

    fn list_packages(&self) {
        println!("Installed packages:");
        for (name, package) in &self.installed_packages {
            println!("{} ({})", name, package.version);
        }
    }

    fn package_info(&self, name: &str) {
        if let Some(package) = self.installed_packages.get(name) {
            println!("Package: {}", package.name);
            println!("Version: {}", package.version);
            println!("Dependencies: {:?}", package.dependencies);
        } else {
            println!("Package not found: {}", name);
        }
    }
}

#[derive(Debug)]
enum SimplexError {
    MissingCommand,
    MissingInstallPackage,
    MissingRemovePackageName,
    MissingRemovePackageVersion,
    MissingInfoPackage,
    IllegalCommand,
}

impl fmt::Display for SimplexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SimplexError::MissingCommand => write!(f, "User did not specify command. Usage: simplex <command> [<args>]"),
            SimplexError::MissingInstallPackage => write!(f, "User did not specify which package to be installed. Usage: simplex install <package-name>"),
            SimplexError::IllegalCommand => write!(f, "User specified an illegal command. For more info about legal commands: simplex --help"),
            SimplexError::MissingRemovePackageName => write!(f, "User did not specify which package to be removed. Usage: simplex remove <package-name> <package-version>"),
            SimplexError::MissingRemovePackageVersion => write!(f, "User did not specify which version of the package to be removed. Usage: simplex remove <package-name> <package-version>"),
            SimplexError::MissingInfoPackage => write!(f, "User did not specify which package to get more information about. Usage: simplex info <package-name>"),
        }
    }
}

impl Error for SimplexError {}

fn run() -> Result<(), Box<dyn Error>> {
    let mut pm = PackageManager::new("./store")?;

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
            let sqlite = Package {
                name: "sqlite".to_string(),
                version: "3.36.0".to_string(),
                dependencies: vec![],
                build_steps: vec![
                    "./configure".to_string(),
                    "make".to_string(),
                    "make install".to_string(),
                ],

                url: "https://www.sqlite.org/2021/sqlite-autoconf-3360000.tar.gz".to_string(),
            };
            pm.fetch_package(&sqlite)?;
            pm.build_package(&sqlite)?;
            pm.install_package(&sqlite)?;
        }
        "remove" => {
            if args.len() < 3 {
                return Err(Box::new(SimplexError::MissingRemovePackageName));
            }
            if args.len() < 4 {
                return Err(Box::new(SimplexError::MissingRemovePackageVersion));
            }
            pm.remove_package(args[3].as_str(), args[4].as_ref())?;
        }
        "list" => {
            pm.list_packages();
        }
        "info" => {
            if args.len() < 3 {
                return Err(Box::new(SimplexError::MissingInfoPackage));
            }
            pm.package_info(&args[2]);
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
