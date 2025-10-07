use std::{process::Command, str::from_utf8};

fn main() {
    let git_output = Command::new("git")
        .arg("describe")
        .arg("--tags")
        .arg("--always")
        .arg("--dirty")
        .output();

    let version = match git_output {
        Ok(output) if output.status.success() => {
            from_utf8(&output.stdout).unwrap().trim().to_string()
        }
        _ => format!("no version info on build"),
    };

    println!("cargo:rustc-env=EPIPHYTE_BUILD_VERSION={}", version);
}
