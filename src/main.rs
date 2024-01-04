use std::env;
use std::io::{self, Write};
use std::process::Command;
use tgs_handler;
use tgs_setup;
use tgs_shell;
use tgs_t5_finetunned;
use tokio::runtime;

fn main() {
    let path = std::env::var("PATH").unwrap();
    tgs_welcome::display_welcome_message();
    let mut runtime = runtime::Runtime::new().unwrap();

    let config = tgs_setup::TgsSetup::new();
    match config.setup() {
        Ok(_) => println!("TGS setup complete."),
        Err(e) => eprintln!("Error setting up TGS: {}", e),
    }

    loop {
        // 1. Print a prompt.
        let current_dir = env::current_dir().unwrap();
        let current_dir_str = current_dir.to_str().unwrap();

        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .output()
            .unwrap();

        let branch_name = String::from_utf8_lossy(&output.stdout)
            .into_owned()
            .trim()
            .to_string();
        print!(
            "{}  {} git:({}) \u{2023} ",
            "\u{27e3}", current_dir_str, branch_name
        );
        io::stdout().flush().unwrap(); // Ensure the prompt is displayed immediately.

        // 2. Read a line of input.
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim(); // Trim whitespace

        // 4. Execute the command using tgs_handler.
        let value: Vec<&str> = input.split(" ").collect();

        let bin = tgs_handler::find_binary(value[0], &path);

        // 5. Execute the command using tgs_shell
        match bin {
            Ok(bin) => {
                let bin_str = bin.to_str().unwrap(); // Convert PathBuf to &str
                match runtime.block_on(tgs_shell::execute(bin_str, &value[1..])) {
                    Ok(exit_status) => {
                        if !exit_status.success() {
                            eprintln!("Command exited with status: {}", exit_status);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Err(e) => {
                // If we didn't find the command, let's see if tgs_t5_finetunned can infer one
                let command_str = value.join(" ");
                match tgs_t5_finetunned::return_command(&command_str) {
                    Ok(inferred_cmd) => {
                        // Now, you'd need to split the inferred command into the binary and arguments
                        // For simplicity, let's assume the inferred command is just a binary name, similar to your initial input
                        // (You might need to modify this to handle complex commands)
                        let inferred_bin = tgs_handler::find_binary(&inferred_cmd, &path);
                        println!("Inferred command: {}", inferred_cmd);
                        match inferred_bin {
                            Ok(inferred_bin_path) => {
                                let bin_str = inferred_bin_path.to_str().unwrap();
                                match runtime.block_on(tgs_shell::execute(bin_str, &value[1..])) {
                                    Ok(exit_status) => {
                                        if !exit_status.success() {
                                            eprintln!(
                                                "Command exited with status: {}",
                                                exit_status
                                            );
                                        }
                                    }
                                    Err(err) => eprintln!("Error: {}", err),
                                }
                            }
                            Err(err) => eprintln!("Error after inference: {}", err),
                        }
                    }
                    Err(err) => eprintln!("Inference error: {}", err),
                }
                eprintln!("Error {}", e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use duct::cmd;

    #[test]
    fn test_tgs_shell_echo() {
        // Simulate user input "echo Hello, world!" and "exit"
        println!("Should echo Hello, world!");
        let input = "echo Hello, world!\nexit\n";
        let output = cmd!("cargo", "run")
            .stdin_bytes(input)
            .read()
            .expect("Failed to run tgs_shell with input");

        // Print the actual output for debugging
        println!("Actual output: {}", output);

        // Check that the output contains the expected prompt and output
        assert!(output.contains("tgs> Hello, world!"));
    }

    #[test]
    fn test_tgs_shell_exit() {
        // Simulate user input "exit"
        println!("Should exit with status 200");
        let input = "exit\n";
        let output = cmd!("cargo", "run")
            .stdin_bytes(input)
            .read()
            .expect("Failed to run tgs_shell with input");

        // Check that the output contains the expected prompt
        assert!(output.contains("tgs> "));
    }
}
