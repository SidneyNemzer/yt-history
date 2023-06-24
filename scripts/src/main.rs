use std::env;
use std::path::Path;
use std::process::Command;
use std::process::Output;

use colored::Colorize;

// Commands:
// - install-dev: Installs a command ythdev that is a shortcut to `cargo run` in the main binary
//
// Any other arguments are passed to `cargo run` for the main binary.

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let scripts_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    if args.get(0) == Some(&"install-dev".to_string()) {
        let default_install_dir = String::from("/usr/local/bin");
        let install_dir = args.get(1).unwrap_or(&default_install_dir);

        install_dev(scripts_dir, &install_dir, false);
        return;
    }

    let project_dir = scripts_dir
        .parent()
        .expect("Failed to get project directory");

    let cargo_output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(project_dir)
        .output()
        .expect("Failed to run cargo");

    if !cargo_output.status.success() {
        let cargo_output_stderr = String::from_utf8_lossy(&cargo_output.stderr);
        print!("{}", cargo_output_stderr);
        println!("{}", "`cargo build` failed for main binary".red());
        std::process::exit(1);
    }

    let exec_path = project_dir.join("target/release/yt-history");

    let s = Command::new(exec_path)
        .args(&args)
        .status()
        .expect("Failed to yt-history binary");

    std::process::exit(s.code().unwrap_or(0));
}

fn install_dev(scripts_dir: &Path, destination: &str, is_retry: bool) {
    let cargo_result = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(scripts_dir)
        .status()
        .expect("Failed to run cargo");

    if !cargo_result.success() {
        println!("{}", "Failed build dev command".red());
        std::process::exit(1);
    }

    let scripts_binary = scripts_dir.join("target/release/scripts");
    let installed_script_path_buf = Path::new(destination).join("ythdev");
    let installed_script_path = installed_script_path_buf.as_path();
    let install_output = install(&scripts_binary, installed_script_path, is_retry);

    let install_output_stderr = String::from_utf8_lossy(&install_output.stderr);
    let is_permission_error = install_output_stderr.find("Permission denied").is_some();

    if !install_output.status.success() {
        if is_permission_error {
            if is_retry {
                print!("{}", install_output_stderr);
                println!("{}", "Failed get sudo access".red());
                std::process::exit(1);
            }

            let sudo_result = Command::new("sudo")
                .arg("--validate")
                .status()
                .expect("Failed to run sudo install command");

            if !sudo_result.success() {
                println!("{}", "Failed to run sudo command".red());
                std::process::exit(1);
            }

            install_dev(scripts_dir, destination, true);
            return;
        } else {
            print!("{}", install_output_stderr);
            println!("{}", "Failed to install dev command".red());
            std::process::exit(1);
        }
    }

    println!(
        "{} {}",
        "Installed dev command to".green(),
        installed_script_path.to_str().unwrap().bold()
    );
}

fn install(scripts_dir: &Path, destination: &Path, sudo: bool) -> Output {
    if sudo {
        return Command::new("sudo")
            .arg("install")
            .arg(scripts_dir)
            .arg(destination)
            .output()
            .expect("Failed to run sudo install command");
    } else {
        return Command::new("install")
            .arg(scripts_dir)
            .arg(destination)
            .output()
            .expect("Failed to run install command");
    }
}
